use std::time::{Duration, Instant};

/// 词法分析性能基准测试
///
/// 对比 GPU、CPU（手写 lexer）、tree-sitter 三种高亮方案的性能。
pub struct LexerBenchmark {
    /// 测试结果
    pub results: Vec<BenchmarkResult>,
}

/// 单次基准测试结果
#[derive(Clone, Debug)]
pub struct BenchmarkResult {
    /// 测试名称
    pub name: String,
    /// 文件大小（字节）
    pub file_size: usize,
    /// 行数
    pub line_count: usize,
    /// 平均耗时
    pub avg_duration: Duration,
    /// 最小耗时
    pub min_duration: Duration,
    /// 最大耗时
    pub max_duration: Duration,
    /// 迭代次数
    pub iterations: usize,
    /// 吞吐量（MB/s）
    pub throughput_mbps: f64,
    /// 延迟（ms/line）
    pub latency_ms_per_line: f64,
}

impl LexerBenchmark {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }

    /// 运行基准测试
    ///
    /// # Arguments
    /// * `name` - 测试名称
    /// * `text` - 测试文本
    /// * `f` - 待测函数
    /// * `iterations` - 迭代次数
    pub fn run<F>(&mut self, name: &str, text: &str, mut f: F, iterations: usize)
    where
        F: FnMut(&str),
    {
        let file_size = text.len();
        let line_count = text.lines().count();

        let mut durations: Vec<Duration> = Vec::with_capacity(iterations);

        for _ in 0..iterations {
            let start = Instant::now();
            f(text);
            let duration = start.elapsed();
            durations.push(duration);
        }

        let avg_duration = durations.iter().sum::<Duration>() / iterations as u32;
        let min_duration = *durations.iter().min().unwrap_or(&Duration::ZERO);
        let max_duration = *durations.iter().max().unwrap_or(&Duration::ZERO);

        let total_secs = durations.iter().sum::<Duration>().as_secs_f64();
        let throughput_mbps = if total_secs > 0.0 {
            (file_size as f64 * iterations as f64) / (total_secs * 1024.0 * 1024.0)
        } else {
            0.0
        };

        let latency_ms_per_line = if iterations > 0 {
            avg_duration.as_secs_f64() * 1000.0 / line_count as f64
        } else {
            0.0
        };

        self.results.push(BenchmarkResult {
            name: name.to_string(),
            file_size,
            line_count,
            avg_duration,
            min_duration,
            max_duration,
            iterations,
            throughput_mbps,
            latency_ms_per_line,
        });
    }

    /// 打印测试报告
    pub fn print_report(&self) {
        println!("\n========== 词法分析性能基准测试 ==========");
        println!(
            "{:20} {:>10} {:>10} {:>12} {:>12} {:>12} {:>10}",
            "方案", "大小(KB)", "行数", "平均(ms)", "最小(ms)", "最大(ms)", "MB/s"
        );
        println!("{}", "-".repeat(90));

        for result in &self.results {
            println!(
                "{:20} {:>10.1} {:>10} {:>12.3} {:>12.3} {:>12.3} {:>10.1}",
                result.name,
                result.file_size as f64 / 1024.0,
                result.line_count,
                result.avg_duration.as_secs_f64() * 1000.0,
                result.min_duration.as_secs_f64() * 1000.0,
                result.max_duration.as_secs_f64() * 1000.0,
                result.throughput_mbps
            );
        }

        println!("\n========== 延迟对比 ==========");
        println!("{:20} {:>15}", "方案", "ms/行");
        println!("{}", "-".repeat(40));
        for result in &self.results {
            println!("{:20} {:>15.6}", result.name, result.latency_ms_per_line);
        }

        // 计算加速比
        if self.results.len() >= 2 {
            println!("\n========== 加速比 ==========");
            let baseline = &self.results[0];
            for result in &self.results[1..] {
                let speedup =
                    baseline.avg_duration.as_secs_f64() / result.avg_duration.as_secs_f64();
                println!("{} vs {}: {:.2}x", result.name, baseline.name, speedup);
            }
        }
    }

    /// 生成 Markdown 格式的报告
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str("# 词法分析性能基准测试\n\n");
        md.push_str("| 方案 | 大小(KB) | 行数 | 平均(ms) | 最小(ms) | 最大(ms) | MB/s |\n");
        md.push_str("|------|----------|------|----------|----------|----------|------|\n");

        for result in &self.results {
            md.push_str(&format!(
                "| {} | {:.1} | {} | {:.3} | {:.3} | {:.3} | {:.1} |\n",
                result.name,
                result.file_size as f64 / 1024.0,
                result.line_count,
                result.avg_duration.as_secs_f64() * 1000.0,
                result.min_duration.as_secs_f64() * 1000.0,
                result.max_duration.as_secs_f64() * 1000.0,
                result.throughput_mbps
            ));
        }

        md
    }
}

/// 生成测试用的代码文本
pub mod test_data {
    /// 生成指定行数的 Rust 代码
    pub fn generate_rust_code(lines: usize) -> String {
        let mut code = String::new();
        let line_template = [
            "pub fn function_name() -> Result<Type, Error> {",
            "    let mut variable = 42;",
            "    // This is a comment",
            "    if condition {",
            "        do_something();",
            "    } else {",
            "        do_other_thing();",
            "    }",
            "    let string = \"hello world\";",
            "    return Ok(variable);",
            "}",
        ];

        for i in 0..lines {
            let line = line_template[i % line_template.len()]
                .replace("function_name", &format!("func_{}", i))
                .replace("variable", &format!("var_{}", i))
                .replace("condition", &format!("cond_{}", i));
            code.push_str(&line);
            code.push('\n');
        }

        code
    }

    /// 生成指定行数的 JavaScript 代码
    pub fn generate_js_code(lines: usize) -> String {
        let mut code = String::new();
        let line_template = [
            "function functionName() {",
            "    const variable = 42;",
            "    // This is a comment",
            "    if (condition) {",
            "        doSomething();",
            "    } else {",
            "        doOtherThing();",
            "    }",
            "    const string = 'hello world';",
            "    return variable;",
            "}",
        ];

        for i in 0..lines {
            let line = line_template[i % line_template.len()]
                .replace("functionName", &format!("func_{}", i))
                .replace("variable", &format!("var_{}", i))
                .replace("condition", &format!("cond_{}", i));
            code.push_str(&line);
            code.push('\n');
        }

        code
    }

    /// 生成指定行数的 JSON
    pub fn generate_json(lines: usize) -> String {
        let mut code = String::new();
        code.push_str("{\n");
        for i in 0..lines {
            code.push_str(&format!(
                "  \"key_{}\": {{ \"name\": \"value_{}\", \"count\": {}, \"active\": true }}",
                i, i, i
            ));
            if i < lines - 1 {
                code.push(',');
            }
            code.push('\n');
        }
        code.push_str("}\n");
        code
    }
}

#[cfg(test)]
mod tests {
    use super::test_data::*;
    use super::*;

    #[test]
    fn test_benchmark_rust() {
        let code = generate_rust_code(1000);
        let mut bench = LexerBenchmark::new();

        // 模拟 CPU lexer
        bench.run(
            "CPU Lexer",
            &code,
            |text| {
                let _ = text.split_whitespace().count();
            },
            100,
        );

        // 模拟 GPU lexer（更快）
        bench.run(
            "GPU Lexer",
            &code,
            |_text| {
                // 模拟 GPU 处理时间
                std::thread::sleep(Duration::from_micros(10));
            },
            100,
        );

        bench.print_report();
        assert!(!bench.results.is_empty());
    }

    #[test]
    fn test_generate_test_data() {
        let rust = generate_rust_code(100);
        assert!(rust.contains("pub fn"));
        assert_eq!(rust.lines().count(), 100);

        let js = generate_js_code(100);
        assert!(js.contains("function"));
        assert_eq!(js.lines().count(), 100);

        let json = generate_json(100);
        assert!(json.contains("\"key_0\""));
    }
}
