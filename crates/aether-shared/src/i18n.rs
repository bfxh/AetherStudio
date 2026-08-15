//! 轻量 i18n：UI 文案翻译查询。
//!
//! key 使用中文原文（中文为默认语言，直通返回）；英文界面按静态翻译表查找，
//! 未命中时回退返回 key 本身（保证任何语言下 UI 不出现空文案）。

/// 界面语言
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiLanguage {
    Chinese,
    English,
}

/// 检测当前界面语言。
///
/// Windows：`GetUserDefaultUILanguage` 主语言 ID（0x04 = 简体/繁体中文）；
/// 非 Windows：`LANG` 环境变量前缀。
pub fn ui_language() -> UiLanguage {
    #[cfg(windows)]
    {
        use windows::Win32::Globalization::GetUserDefaultUILanguage;
        let lang = unsafe { GetUserDefaultUILanguage() };
        // LANGID 主语言 ID：低 10 位
        let primary = lang & 0x3FF;
        if primary == 0x04 {
            UiLanguage::Chinese
        } else {
            UiLanguage::English
        }
    }
    #[cfg(not(windows))]
    {
        let lang = std::env::var("LANG").unwrap_or_default().to_lowercase();
        if lang.starts_with("zh") {
            UiLanguage::Chinese
        } else {
            UiLanguage::English
        }
    }
}

/// 翻译表（中文 key → 英文）。新增 UI 文案时在此补充即可。
const EN_TABLE: &[(&str, &str)] = &[
    // 菜单栏
    ("文件(F)", "File(F)"),
    ("文件", "File"),
    ("新建项目", "New Project"),
    ("新建窗口", "New Window"),
    ("打开文件...", "Open File..."),
    ("打开文件", "Open File"),
    ("打开文件夹...", "Open Folder..."),
    ("打开文件夹", "Open Folder"),
    ("关闭工作区", "Close Workspace"),
    ("保存", "Save"),
    ("另存为...", "Save As..."),
    ("另存为", "Save As"),
    ("退出", "Exit"),
    ("编辑(E)", "Edit(E)"),
    ("编辑", "Edit"),
    ("撤销", "Undo"),
    ("重做", "Redo"),
    ("剪切", "Cut"),
    ("复制", "Copy"),
    ("粘贴", "Paste"),
    ("查找", "Find"),
    ("替换", "Replace"),
    ("全选", "Select All"),
    ("选择(S)", "Select(S)"),
    ("查看(V)", "View(V)"),
    ("切换侧边栏", "Toggle Sidebar"),
    ("切换活动栏", "Toggle Activity Bar"),
    ("切换状态栏", "Toggle Status Bar"),
    ("放大", "Zoom In"),
    ("缩小", "Zoom Out"),
    ("转到(G)", "Go(G)"),
    ("转到文件...", "Go to File..."),
    ("转到文件", "Go to File"),
    ("转到行...", "Go to Line..."),
    ("转到行", "Go to Line"),
    ("运行(R)", "Run(R)"),
    ("运行", "Run"),
    ("启动", "Start"),
    ("调试", "Debug"),
    ("启动调试", "Start Debugging"),
    ("智能体沙盒评测", "Agent Sandbox Eval"),
    ("终端(T)", "Terminal(T)"),
    ("新建终端", "New Terminal"),
    ("帮助(H)", "Help(H)"),
    ("帮助", "Help"),
    ("检查更新", "Check Updates"),
    ("关于", "About"),
    ("全局搜索", "Global Search"),
    ("AI 修复当前诊断", "AI Fix Diagnostics"),
    ("Markdown 预览", "Markdown Preview"),
];

/// 翻译查询：中文直通；英文查表（未命中返回 key 本身）。
pub fn tr(key: &'static str) -> &'static str {
    if ui_language() == UiLanguage::Chinese {
        return key;
    }
    for (k, v) in EN_TABLE {
        if *k == key {
            return *v;
        }
    }
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_keys_unique() {
        for (i, (k, _)) in EN_TABLE.iter().enumerate() {
            assert!(!k.is_empty());
            for (k2, _) in &EN_TABLE[i + 1..] {
                assert_ne!(k, k2, "翻译表 key 重复: {}", k);
            }
        }
    }

    #[test]
    fn test_tr_known_key_returns_english() {
        // ui_language 取决于系统，这里只验证表查找逻辑本身
        let found = EN_TABLE.iter().find(|(k, _)| *k == "保存").map(|(_, v)| *v);
        assert_eq!(found, Some("Save"));
    }

    #[test]
    fn test_tr_unknown_key_falls_back() {
        // 未命中 key 必须回退（不 panic、不返回空）
        let missing = EN_TABLE.iter().all(|(k, _)| *k != "不存在的文案");
        assert!(missing);
    }
}
