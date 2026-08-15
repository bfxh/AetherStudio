//! 状态栏 Language 分区点击弹出的语言模式选择菜单（纯逻辑，可单测）。
//!
//! 交互：点击状态栏 Language 分区 → 打开菜单 → 选择语言 → 切换当前文档语言模式
//! （重新词法高亮 / 触发 LSP 重连）。

use aether_core::lexer::Language;

/// 语言选择菜单项：显示名 + 目标语言
pub struct LanguageMenuItem {
    pub label: &'static str,
    pub lang: Language,
}

/// 语言模式选择菜单状态（仿 activity_bar_context_menu 的纯逻辑状态）
#[derive(Clone, Debug)]
pub struct LanguageMenuState {
    /// 是否可见
    pub visible: bool,
    /// 菜单位置（左上角）
    pub x: f32,
    pub y: f32,
    /// 当前 hover 项索引
    pub hover_index: Option<usize>,
}

impl Default for LanguageMenuState {
    fn default() -> Self {
        Self {
            visible: false,
            x: 0.0,
            y: 0.0,
            hover_index: None,
        }
    }
}

impl LanguageMenuState {
    /// 单项高度
    pub const ITEM_HEIGHT: f32 = 24.0;
    /// 菜单宽度
    pub const MENU_WIDTH: f32 = 180.0;
    /// 顶部 padding
    pub const TOP_PADDING: f32 = 4.0;
    /// 底部 padding
    pub const BOTTOM_PADDING: f32 = 4.0;

    /// 打开菜单：位置为状态栏分区的锚点（菜单向上弹出）
    pub fn open_at(&mut self, x: f32, y: f32) {
        self.visible = true;
        self.x = x;
        self.y = y;
        self.hover_index = None;
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.hover_index = None;
    }

    pub fn menu_height(&self) -> f32 {
        Self::TOP_PADDING + Self::BOTTOM_PADDING + language_options().len() as f32 * Self::ITEM_HEIGHT
    }

    /// 命中测试：返回命中的菜单项索引（相对菜单左上角）
    pub fn hit_test(&self, x: f32, y: f32) -> Option<usize> {
        if !self.visible {
            return None;
        }
        if x < self.x || x >= self.x + Self::MENU_WIDTH {
            return None;
        }
        // 减去顶部 padding：与渲染起点对齐，padding 区域不算命中
        let rel_y = y - self.y - Self::TOP_PADDING;
        if rel_y < 0.0 {
            return None;
        }
        let idx = (rel_y / Self::ITEM_HEIGHT) as usize;
        if idx < language_options().len() {
            Some(idx)
        } else {
            None
        }
    }

    /// 更新 hover：返回是否变化（用于脏区重绘）
    pub fn update_hover(&mut self, x: f32, y: f32) -> bool {
        let new_hover = self.hit_test(x, y);
        if new_hover != self.hover_index {
            self.hover_index = new_hover;
            true
        } else {
            false
        }
    }
}

/// 语言选择选项（单一来源：aether-core 语言注册表的子集——支持手写高亮的语言）
pub fn language_options() -> &'static [LanguageMenuItem] {
    &[
        LanguageMenuItem { label: "Rust", lang: Language::Rust },
        LanguageMenuItem { label: "Python", lang: Language::Python },
        LanguageMenuItem { label: "C", lang: Language::C },
        LanguageMenuItem { label: "C++", lang: Language::Cpp },
        LanguageMenuItem { label: "JavaScript", lang: Language::JavaScript },
        LanguageMenuItem { label: "TypeScript", lang: Language::TypeScript },
        LanguageMenuItem { label: "Go", lang: Language::Go },
        LanguageMenuItem { label: "Java", lang: Language::Java },
        LanguageMenuItem { label: "JSON", lang: Language::Json },
        LanguageMenuItem { label: "Markdown", lang: Language::Markdown },
        LanguageMenuItem { label: "TOML", lang: Language::Toml },
        LanguageMenuItem { label: "HTML", lang: Language::Html },
        LanguageMenuItem { label: "CSS", lang: Language::Css },
        LanguageMenuItem { label: "Plain Text", lang: Language::PlainText },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_options_unique_languages() {
        let opts = language_options();
        assert!(opts.len() >= 14);
        for (i, o) in opts.iter().enumerate() {
            assert!(!o.label.is_empty());
            for other in &opts[i + 1..] {
                assert_ne!(o.lang, other.lang, "语言重复: {}", o.label);
            }
        }
    }

    #[test]
    fn test_hit_test_invisible() {
        let m = LanguageMenuState::default();
        assert_eq!(m.hit_test(10.0, 10.0), None);
    }

    #[test]
    fn test_hit_test_visible() {
        let mut m = LanguageMenuState::default();
        m.open_at(100.0, 200.0);
        // padding 区内不命中
        assert_eq!(m.hit_test(100.0, 200.0), None);
        // 第一项（y + TOP_PADDING 起）
        assert_eq!(m.hit_test(100.0, 204.0), Some(0));
        // 第二项
        assert_eq!(m.hit_test(100.0, 228.0), Some(1));
        // 超出宽度
        assert_eq!(m.hit_test(100.0 + LanguageMenuState::MENU_WIDTH, 200.0), None);
        // 超出高度（菜单之外）
        let last = language_options().len() as f32 * LanguageMenuState::ITEM_HEIGHT;
        assert_eq!(m.hit_test(100.0, 200.0 + last), None);
    }

    #[test]
    fn test_update_hover() {
        let mut m = LanguageMenuState::default();
        m.open_at(0.0, 0.0);
        assert!(m.update_hover(5.0, 10.0)); // None -> Some(0)
        assert!(!m.update_hover(5.0, 10.0)); // 无变化
        assert!(m.update_hover(5.0, 34.0)); // Some(0) -> Some(1)
        m.hide();
        assert!(m.hover_index.is_none());
    }
}
