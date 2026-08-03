//! 后台语法高亮器
//!
//! 将 Tree-sitter 解析和高亮移到后台线程，避免阻塞 UI 输入。
//!
//! 工作流程：
//! 1. 主线程调用 `request()` 发送高亮请求（轻量缓冲区快照 + 语言 + 版本号）
//! 2. 后台线程接收请求，在后台物化全文后调用 `highlight_document`
//! 3. 主线程在渲染帧中调用 `poll_result()` 非阻塞检查结果
//! 4. 结果未就绪时使用上一帧的缓存（无卡顿）
//!
//! P1-C: 请求携带 `TextBufferSnapshot`（Arc 共享的 piece 列表，轻量）而非
//! 全文 String，避免每次编辑在 UI 线程上做全文拷贝；文本物化移到后台线程。
//!
//! 卡顿修复（P0/P1）：
//! - 结果缓存：按 (doc_id, version) 缓存 token_lines（LRU 淘汰），切回已打开的
//!   标签页时零解析直接复用，消除"每次切换标签都重新解析整个文件"的卡顿；
//! - 最新请求覆盖：worker 排空队列只处理最新请求，快速连续点击时不排队积压，
//!   高亮及时跟上（旧请求结果由主线程侧按 doc_id/version 匹配校验丢弃）。

use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;

use aether_core::buffer::text_buffer::TextBufferSnapshot;
use aether_core::lexer::LexemeSpan;

use crate::highlighter::TreeSitterHighlighter;

/// 高亮请求
struct HighlightRequest {
    doc_id: String,
    language: String,
    /// 缓冲区版本号：缓存键的一部分，编辑后版本变化强制重新解析
    version: u64,
    /// 缓冲区快照（UI 线程零拷贝，后台线程物化全文）
    snapshot: Box<dyn TextBufferSnapshot>,
}

/// 高亮结果
pub struct HighlightResult {
    pub doc_id: String,
    /// 与请求对应的缓冲区版本号（主线程据此校验结果归属）
    pub version: u64,
    pub token_lines: Vec<Vec<LexemeSpan>>,
}

/// 缓存容量：最多缓存 4 个文档（10 万行文档 tokens 约数 MB，LRU 淘汰控制内存）
const MAX_CACHED_DOCS: usize = 4;

/// 后台语法高亮器
///
/// 拥有独立的后台线程，线程内持有专属的 `TreeSitterHighlighter` 实例与
/// (doc_id, version) → token_lines 结果缓存。主线程通过 channel 与后台线程
/// 通信，完全不阻塞。
pub struct BackgroundHighlighter {
    /// 请求发送端（主线程持有）
    request_tx: Sender<HighlightRequest>,
    /// 结果接收端（主线程持有）
    result_rx: Receiver<HighlightResult>,
    /// 后台线程句柄
    _worker: Option<thread::JoinHandle<()>>,
    /// 是否有请求在途（结果尚未被主线程消费）
    pending: bool,
}

impl BackgroundHighlighter {
    /// 创建并启动后台高亮器
    pub fn new() -> Self {
        let (req_tx, req_rx) = mpsc::channel::<HighlightRequest>();
        let (res_tx, res_rx) = mpsc::channel::<HighlightResult>();

        let worker = thread::spawn(move || {
            let mut highlighter = TreeSitterHighlighter::new();
            // (doc_id, version) → token_lines 结果缓存（LRU 淘汰）
            let mut cache: HashMap<(String, u64), Vec<Vec<LexemeSpan>>> = HashMap::new();
            let mut cache_order: VecDeque<(String, u64)> = VecDeque::new();

            loop {
                // 阻塞等待请求；发送端断开（主线程关闭）时退出
                let mut req = match req_rx.recv() {
                    Ok(r) => r,
                    Err(_) => break,
                };
                // 只处理最新请求：排空队列中堆积的旧请求。
                // 快速连续点击/切换标签时只解析最后一次，避免排队积压导致
                // 高亮延迟累积（被丢弃请求的结果也永远不会产生）。
                while let Ok(newer) = req_rx.try_recv() {
                    req = newer;
                }

                let key = (req.doc_id.clone(), req.version);
                let token_lines = if let Some(tokens) = cache.get(&key) {
                    // 缓存命中：零解析直接复用（刷新 LRU 顺序）
                    if let Some(pos) = cache_order.iter().position(|k| k == &key) {
                        cache_order.remove(pos);
                    }
                    cache_order.push_back(key);
                    tokens.clone()
                } else {
                    // 缓存未命中：物化全文并解析，结果存入缓存
                    let full_text = req.snapshot.full_text();
                    let tokens =
                        highlighter.highlight_document(&req.doc_id, &req.language, &full_text);
                    if cache.len() >= MAX_CACHED_DOCS {
                        if let Some(oldest) = cache_order.pop_front() {
                            cache.remove(&oldest);
                        }
                    }
                    cache.insert(key.clone(), tokens.clone());
                    cache_order.push_back(key);
                    tokens
                };

                // 结果发送失败表示主线程已关闭，退出循环
                if res_tx
                    .send(HighlightResult {
                        doc_id: req.doc_id,
                        version: req.version,
                        token_lines,
                    })
                    .is_err()
                {
                    break;
                }
            }
        });

        Self {
            request_tx: req_tx,
            result_rx: res_rx,
            _worker: Some(worker),
            pending: false,
        }
    }

    /// 发送高亮请求（非阻塞）
    ///
    /// 总是发送：worker 内部排空旧请求只处理最新，无需 pending 拦截；
    /// `pending` 仅用于主线程判断"结果是否已消费"（驱动高亮刷新定时器）。
    pub fn request(
        &mut self,
        doc_id: &str,
        language: &str,
        version: u64,
        snapshot: Box<dyn TextBufferSnapshot>,
    ) {
        let _ = self.request_tx.send(HighlightRequest {
            doc_id: doc_id.to_string(),
            language: language.to_string(),
            version,
            snapshot,
        });
        self.pending = true;
    }

    /// 非阻塞轮询高亮结果
    ///
    /// 返回 `Some(result)` 表示有新结果就绪；返回 `None` 表示仍在处理中，
    /// 主线程应使用上一帧缓存。结果归属校验（doc_id/version 是否匹配当前
    /// 活动文件）由主线程完成，不匹配时直接丢弃。
    pub fn poll_result(&mut self) -> Option<HighlightResult> {
        match self.result_rx.try_recv() {
            Ok(result) => {
                self.pending = false;
                // 排空可能残留的旧结果
                while self.result_rx.try_recv().is_ok() {}
                Some(result)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.pending = false;
                None
            }
        }
    }

    /// 是否有待处理请求（结果尚未被消费）
    pub fn has_pending(&self) -> bool {
        self.pending
    }
}

impl Default for BackgroundHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aether_core::buffer::text_buffer::TextBufferSnapshot;
    use std::time::{Duration, Instant};

    struct MockSnapshot(String);

    impl TextBufferSnapshot for MockSnapshot {
        fn slice(&self, start: usize, end: usize) -> String {
            self.0[start.min(self.0.len())..end.min(self.0.len())].to_string()
        }
        fn full_text(&self) -> String {
            self.0.clone()
        }
        fn line_count(&self) -> usize {
            self.0.lines().count()
        }
        fn line_text(&self, line_idx: usize) -> Option<String> {
            self.0.lines().nth(line_idx).map(|s| s.to_string())
        }
        fn byte_len(&self) -> usize {
            self.0.len()
        }
    }

    fn poll_until(hl: &mut BackgroundHighlighter, timeout_ms: u64) -> Option<HighlightResult> {
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        while Instant::now() < deadline {
            if let Some(r) = hl.poll_result() {
                return Some(r);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        None
    }

    #[test]
    fn test_cache_hit_returns_quickly() {
        let mut hl = BackgroundHighlighter::new();
        let text = (0..2000)
            .map(|i| format!("fn f{}() -> i32 {{ {} }}", i, i))
            .collect::<Vec<_>>()
            .join("\n");

        // 第一次请求：真实解析（2000 行耗时明显）
        hl.request("doc1", "rust", 1, Box::new(MockSnapshot(text.clone())));
        let t0 = Instant::now();
        let r1 = poll_until(&mut hl, 10_000).expect("首次解析应有结果");
        let first_ms = t0.elapsed().as_millis();
        assert_eq!(r1.doc_id, "doc1");
        assert_eq!(r1.version, 1);

        // 第二次相同 (doc_id, version)：缓存命中，应远快于首次
        hl.request("doc1", "rust", 1, Box::new(MockSnapshot(text)));
        let t0 = Instant::now();
        let r2 = poll_until(&mut hl, 2_000).expect("缓存命中应有结果");
        let second_ms = t0.elapsed().as_millis();
        assert_eq!(r2.token_lines.len(), r1.token_lines.len());
        assert!(
            second_ms < first_ms / 3,
            "缓存命中应显著快于重新解析（first={}ms, second={}ms）",
            first_ms,
            second_ms
        );
    }

    #[test]
    fn test_version_change_reparses() {
        let mut hl = BackgroundHighlighter::new();
        hl.request(
            "doc1",
            "rust",
            1,
            Box::new(MockSnapshot("fn main() {}\nfn second() {}".to_string())),
        );
        let r1 = poll_until(&mut hl, 10_000).unwrap();

        // 编辑后 buffer_version 变化 → 必须重新解析（不同内容产生不同 tokens）
        hl.request(
            "doc1",
            "rust",
            2,
            Box::new(MockSnapshot("let x = 42; // comment".to_string())),
        );
        let r2 = poll_until(&mut hl, 10_000).unwrap();
        assert_eq!(r2.version, 2);
        assert_ne!(r1.token_lines.len(), r2.token_lines.len());
    }

    #[test]
    fn test_latest_request_wins() {
        let mut hl = BackgroundHighlighter::new();
        // 快速连发 3 个请求（不 poll）：worker 应只处理最新的 doc2
        hl.request(
            "doc0",
            "rust",
            1,
            Box::new(MockSnapshot("fn a() {}".to_string())),
        );
        hl.request(
            "doc1",
            "rust",
            1,
            Box::new(MockSnapshot("fn b() {}".to_string())),
        );
        hl.request(
            "doc2",
            "rust",
            1,
            Box::new(MockSnapshot("fn c() {}".to_string())),
        );

        let r = poll_until(&mut hl, 10_000).expect("应有结果");
        assert_eq!(r.doc_id, "doc2", "旧请求应被丢弃，只返回最新请求的结果");
        assert!(hl.poll_result().is_none(), "不应残留被丢弃请求的结果");
    }

    #[test]
    fn test_lru_eviction_keeps_working() {
        let mut hl = BackgroundHighlighter::new();
        let texts: Vec<String> = (0..5)
            .map(|i| format!("// doc{}\nfn main() {{ {} }}", i, i))
            .collect();

        // 填满缓存容量（MAX_CACHED_DOCS = 4）
        for i in 0..4 {
            hl.request(
                &format!("doc{}", i),
                "rust",
                1,
                Box::new(MockSnapshot(texts[i].clone())),
            );
            poll_until(&mut hl, 10_000).unwrap();
        }
        // 第 5 个文档触发 LRU 淘汰
        hl.request("doc4", "rust", 1, Box::new(MockSnapshot(texts[4].clone())));
        poll_until(&mut hl, 10_000).unwrap();
        // 被淘汰的 doc0 重新请求仍能正确返回结果（重新解析）
        hl.request("doc0", "rust", 1, Box::new(MockSnapshot(texts[0].clone())));
        let r = poll_until(&mut hl, 10_000).unwrap();
        assert_eq!(r.doc_id, "doc0");
        assert_eq!(r.token_lines.len(), 2);
    }
}
