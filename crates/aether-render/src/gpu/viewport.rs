use aether_core::lexer::LexemeSpan;

/// 视口优先的增量高亮缓存
///
/// 只缓存可见行范围内的高亮结果，配合编辑距离检测实现增量更新。
#[derive(Clone, Debug)]
pub struct ViewportHighlightCache {
    /// 缓存窗口起始行（全局行号）
    pub window_start: usize,
    /// 缓存窗口大小（行数）
    pub window_len: usize,
    /// 每行的 token 列表（下标 = 全局行号 - window_start）
    pub tokens: Vec<Vec<LexemeSpan>>,
    /// 每行的 buffer_version（用于检测过期）
    pub versions: Vec<u64>,
    /// 当前 buffer_version（外部传入）
    pub current_version: u64,
    /// 脏行标记（需要重新高亮）
    pub dirty_lines: Vec<bool>,
    /// 每行的文本内容（用于编辑距离比较）
    pub line_texts: Vec<String>,
}

impl ViewportHighlightCache {
    pub fn new() -> Self {
        Self {
            window_start: 0,
            window_len: 0,
            tokens: Vec::new(),
            versions: Vec::new(),
            current_version: 0,
            dirty_lines: Vec::new(),
            line_texts: Vec::new(),
        }
    }

    /// 获取指定全局行的 token 列表
    pub fn get_line_tokens(&self, line_idx: usize) -> Option<&[LexemeSpan]> {
        let slot = line_idx.checked_sub(self.window_start)?;
        self.tokens.get(slot).map(|v| v.as_slice())
    }

    /// 设置指定全局行的 token 列表
    pub fn set_line_tokens(&mut self, line_idx: usize, tokens: Vec<LexemeSpan>, version: u64) {
        if let Some(slot) = line_idx.checked_sub(self.window_start) {
            if slot < self.tokens.len() {
                self.tokens[slot] = tokens;
                self.versions[slot] = version;
                self.dirty_lines[slot] = false;
            }
        }
    }

    /// 标记指定行为脏（需要重新高亮）
    pub fn mark_line_dirty(&mut self, line_idx: usize) {
        if let Some(slot) = line_idx.checked_sub(self.window_start) {
            if slot < self.dirty_lines.len() {
                self.dirty_lines[slot] = true;
            }
        }
    }

    /// 调整缓存窗口大小和位置
    ///
    /// 重叠部分保留，新进入窗口的行标记为脏。
    pub fn resize_window(&mut self, new_start: usize, new_len: usize, version: u64) {
        if self.window_start == new_start && self.window_len == new_len && version == self.current_version {
            return;
        }

        self.current_version = version;

        // 创建新的缓存
        let mut new_tokens: Vec<Vec<LexemeSpan>> = Vec::with_capacity(new_len);
        let mut new_versions: Vec<u64> = Vec::with_capacity(new_len);
        let mut new_dirty: Vec<bool> = Vec::with_capacity(new_len);
        let mut new_texts: Vec<String> = Vec::with_capacity(new_len);

        for gi in new_start..new_start + new_len {
            if gi >= self.window_start && gi < self.window_start + self.window_len {
                // 重叠行：保留旧数据
                let old_slot = gi - self.window_start;
                new_tokens.push(std::mem::take(&mut self.tokens[old_slot]));
                new_versions.push(self.versions[old_slot]);
                new_texts.push(std::mem::take(&mut self.line_texts[old_slot]));
                // 如果版本过期，标记为脏
                new_dirty.push(self.versions[old_slot] != version);
            } else {
                // 新行：初始化为空，标记为脏
                new_tokens.push(Vec::new());
                new_versions.push(0);
                new_dirty.push(true);
                new_texts.push(String::new());
            }
        }

        self.window_start = new_start;
        self.window_len = new_len;
        self.tokens = new_tokens;
        self.versions = new_versions;
        self.dirty_lines = new_dirty;
        self.line_texts = new_texts;
    }

    /// 使用编辑距离检测增量更新
    ///
    /// 比较新旧文本，只标记真正发生变化的行为脏。
    pub fn update_with_edit_distance(
        &mut self,
        lines: &[String],
        version: u64,
        threshold: f32,
    ) {
        self.current_version = version;

        for (slot, new_text) in lines.iter().enumerate() {
            if slot >= self.line_texts.len() {
                break;
            }

            let old_text = &self.line_texts[slot];

            // 快速路径：文本完全相同
            if old_text == new_text {
                // 保持现有 token，只更新版本
                self.versions[slot] = version;
                self.dirty_lines[slot] = false;
                continue;
            }

            // 检查是否跨越 token 边界
            if EditDistanceDetector::crosses_token_boundary(old_text, new_text) {
                self.dirty_lines[slot] = true;
                self.line_texts[slot] = new_text.clone();
                continue;
            }

            // 计算编辑距离
            let dist = EditDistanceDetector::distance(old_text, new_text);
            let max_len = old_text.len().max(new_text.len());

            if max_len > 0 {
                let ratio = dist as f32 / max_len as f32;
                if ratio > threshold {
                    // 显著变化：标记为脏
                    self.dirty_lines[slot] = true;
                    self.line_texts[slot] = new_text.clone();
                } else {
                    // 微小变化：尝试复用现有 token（偏移调整）
                    // 标记为脏以重新高亮（简化实现）
                    self.dirty_lines[slot] = true;
                    self.line_texts[slot] = new_text.clone();
                }
            }
        }
    }

    /// 获取所有脏行的索引（全局行号）
    pub fn dirty_line_indices(&self) -> Vec<usize> {
        self.dirty_lines
            .iter()
            .enumerate()
            .filter(|(_, &is_dirty)| is_dirty)
            .map(|(slot, _)| self.window_start + slot)
            .collect()
    }

    /// 获取缓存窗口起始行
    pub fn window_start(&self) -> usize {
        self.window_start
    }

    /// 获取缓存窗口大小
    pub fn window_len(&self) -> usize {
        self.window_len
    }

    /// 获取当前缓存的 buffer_version
    pub fn buffer_version(&self) -> u64 {
        self.current_version
    }

    /// 检查缓存是否为空（未初始化）
    pub fn is_empty(&self) -> bool {
        self.window_len == 0 || self.tokens.is_empty()
    }

    /// 清除所有缓存
    pub fn clear(&mut self) {
        self.window_start = 0;
        self.window_len = 0;
        self.tokens.clear();
        self.versions.clear();
        self.dirty_lines.clear();
        self.line_texts.clear();
    }
}

/// 编辑距离检测器
///
/// 检测两行文本之间的编辑距离，用于判断是否需要重新高亮。
pub struct EditDistanceDetector;

impl EditDistanceDetector {
    /// 计算两个字符串的 Levenshtein 编辑距离
    pub fn distance(a: &str, b: &str) -> usize {
        let a_len = a.chars().count();
        let b_len = b.chars().count();

        if a_len == 0 {
            return b_len;
        }
        if b_len == 0 {
            return a_len;
        }

        // 使用滚动数组优化空间
        let mut prev = vec![0; b_len + 1];
        let mut curr = vec![0; b_len + 1];

        for j in 0..=b_len {
            prev[j] = j;
        }

        for (i, a_ch) in a.chars().enumerate() {
            curr[0] = i + 1;
            for (j, b_ch) in b.chars().enumerate() {
                let cost = if a_ch == b_ch { 0 } else { 1 };
                curr[j + 1] = (prev[j + 1] + 1) // 删除
                    .min(curr[j] + 1) // 插入
                    .min(prev[j] + cost); // 替换
            }
            std::mem::swap(&mut prev, &mut curr);
        }

        prev[b_len]
    }

    /// 判断文本变化是否"显著"（需要重新词法分析）
    ///
    /// 策略：
    /// - 编辑距离 > 阈值：需要重新分析
    /// - 仅空白字符变化：不需要重新分析
    /// - 新增/删除字符串/注释：需要重新分析
    pub fn is_significant_change(old_text: &str, new_text: &str) -> bool {
        // 快速路径：完全相同
        if old_text == new_text {
            return false;
        }

        // 快速路径：长度差异过大
        let old_len = old_text.len();
        let new_len = new_text.len();
        if old_len == 0 || new_len == 0 {
            return true;
        }

        // 计算编辑距离
        let dist = Self::distance(old_text, new_text);
        let max_len = old_len.max(new_len);

        // 阈值：编辑距离超过文本长度的 30% 视为显著变化
        let threshold = (max_len as f32 * 0.3) as usize;
        if dist > threshold {
            return true;
        }

        // 检查是否跨越了 token 边界（如引号、注释符号）
        if Self::crosses_token_boundary(old_text, new_text) {
            return true;
        }

        false
    }

    /// 检测变化是否跨越了 token 边界
    ///
    /// 例如：在字符串中间插入引号会改变整个行的 token 结构
    pub fn crosses_token_boundary(old_text: &str, new_text: &str) -> bool {
        // 检查引号数量奇偶性变化
        let old_quotes = old_text.chars().filter(|&c| c == '"' || c == '\'').count();
        let new_quotes = new_text.chars().filter(|&c| c == '"' || c == '\'').count();
        if old_quotes != new_quotes {
            return true;
        }

        // 检查注释符号变化
        let old_comment = old_text.contains("//") || old_text.contains("/*");
        let new_comment = new_text.contains("//") || new_text.contains("/*");
        if old_comment != new_comment {
            return true;
        }

        false
    }
}

/// GPU 高亮管线配置
///
/// 控制 GPU 词法分析的启停和降级策略。
#[derive(Clone, Copy, Debug)]
pub struct GpuHighlightConfig {
    /// 是否启用 GPU 加速
    pub enabled: bool,
    /// 文件大小阈值（字节）：超过此值才使用 GPU
    pub min_file_size: usize,
    /// 视口扩展行数（可见行上下各扩展多少行）
    pub viewport_padding: usize,
    /// 编辑距离阈值（0.0-1.0）
    pub edit_distance_threshold: f32,
    /// 是否回退到 CPU 高亮（GPU 失败时）
    pub fallback_to_cpu: bool,
}

impl Default for GpuHighlightConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_file_size: 1024, // 1KB 以上文件使用 GPU
            viewport_padding: 5, // 可见行上下各扩展 5 行
            edit_distance_threshold: 0.3,
            fallback_to_cpu: true,
        }
    }
}
