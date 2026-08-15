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
    ("语言模式", "Language Mode"),
    ("0 错误 0 警告", "0 Errors 0 Warnings"),
    ("AI: 修复当前诊断", "AI: Fix Diagnostics"),
    (
        "Aether Studio — 纯 Rust 原生编辑器",
        "Aether Studio — Pure Rust Native Editor",
    ),
    ("Markdown: 切换预览", "Markdown: Toggle Preview"),
    (
        "[H-14] D2D 操作失败 (设备丢失?): {:?}",
        "[H-14] D2D operation failed (device lost?): {:?}",
    ),
    ("保", "S"),
    ("保存当前文件", "Save Current File"),
    ("克隆仓库", "Clone Repository"),
    ("关于 Aether 编辑器", "About Aether Editor"),
    ("剪切选中文本", "Cut Selected Text"),
    ("启动调试器", "Start Debugger"),
    ("在工作区中搜索文本", "Search Text in Workspace"),
    ("在文件中查找", "Find in File"),
    (
        "在用户文档目录下创建新项目文件夹",
        "Create New Project Folder in User Documents",
    ),
    (
        "在编辑和预览模式之间切换",
        "Toggle Between Edit and Preview Modes",
    ),
    (
        "在资源管理器中双击文件开始编辑",
        "Double-Click a File in Explorer to Start Editing",
    ),
    (
        "在隔离沙盒中评测 AI 智能体的任务完成度",
        "Evaluate AI Agent Task Completion in Isolated Sandbox",
    ),
    ("复制选中文本", "Copy Selected Text"),
    ("将文件保存到新位置", "Save File to a New Location"),
    ("就绪", "Ready"),
    ("帮助: 关于", "Help: About"),
    ("帮助: 检查更新", "Help: Check Updates"),
    ("快速打开文件", "Quick Open File"),
    ("打开文件夹  Ctrl + K", "Open Folder  Ctrl + K"),
    ("打开文件夹作为工作区", "Open Folder as Workspace"),
    ("打开文件夹开始编辑", "Open Folder to Start Editing"),
    ("打开现有文件", "Open Existing File"),
    ("打开集成终端", "Open Integrated Terminal"),
    (
        "把当前文件的 LSP 错误发送给 AI 修复",
        "Send Current File's LSP Errors to AI for Fixing",
    ),
    ("搜索: 全局搜索", "Search: Global Search"),
    ("撤销上一步操作", "Undo Last Operation"),
    ("文件: 保存", "File: Save"),
    ("文件: 另存为", "File: Save As"),
    ("文件: 打开文件", "File: Open File"),
    ("文件: 打开文件夹", "File: Open Folder"),
    ("文件: 新建项目", "File: New Project"),
    ("文件: 退出", "File: Exit"),
    ("新建项目  Ctrl + N", "New Project  Ctrl + N"),
    ("显示/隐藏侧边栏", "Toggle Sidebar"),
    ("显示/隐藏活动栏", "Toggle Activity Bar"),
    ("显示/隐藏状态栏", "Toggle Status Bar"),
    ("暂无最近项目", "No Recent Projects"),
    ("更多...", "More..."),
    ("最近项目", "Recent Projects"),
    ("查找并替换", "Find and Replace"),
    ("检查并安装新版本", "Check and Install New Version"),
    ("牧羊人编辑器", "Aether Editor"),
    ("粘贴剪贴板内容", "Paste Clipboard Content"),
    ("终端: 新建终端", "Terminal: New Terminal"),
    ("编辑: 全选", "Edit: Select All"),
    ("编辑: 剪切", "Edit: Cut"),
    ("编辑: 复制", "Edit: Copy"),
    ("编辑: 撤销", "Edit: Undo"),
    ("编辑: 替换", "Edit: Replace"),
    ("编辑: 查找", "Edit: Find"),
    ("编辑: 粘贴", "Edit: Paste"),
    ("编辑: 重做", "Edit: Redo"),
    ("视图: 切换侧边栏", "View: Toggle Sidebar"),
    ("视图: 切换活动栏", "View: Toggle Activity Bar"),
    ("视图: 切换状态栏", "View: Toggle Status Bar"),
    ("跳转到指定行", "Jump to Specified Line"),
    ("转到: 转到文件", "Go: Go to File"),
    ("转到: 转到行", "Go: Go to Line"),
    ("运行: 启动", "Run: Start"),
    ("运行: 智能体沙盒评测", "Run: Agent Sandbox Eval"),
    ("运行: 调试", "Run: Debug"),
    ("运行当前项目", "Run Current Project"),
    ("退出编辑器", "Exit Editor"),
    ("选择全部内容", "Select All Content"),
    ("通过 SSH 连接", "Connect via SSH"),
    ("重做已撤销的操作", "Redo Undone Operation"),
    ("黑洞", "Black Hole"),
    (
        "💡 提示：按 Ctrl+K 快速打开文件夹，Ctrl+N 新建项目",
        "Tip: Press Ctrl+K to Open Folder Quickly, Ctrl+N for New Project",
    ),
    ("AI 修复当前诊断", "AI Fix Diagnostics"),
    ("Markdown 预览", "Markdown Preview"),
];

/// 按指定语言翻译（纯函数，可测）：中文直通；英文查表（未命中返回 key 本身）。
pub fn tr_in(lang: UiLanguage, key: &'static str) -> &'static str {
    if lang == UiLanguage::Chinese {
        return key;
    }
    for (k, v) in EN_TABLE {
        if *k == key {
            return *v;
        }
    }
    key
}

/// 翻译查询：按当前系统语言翻译（中文直通；英文查表；未命中回退 key）。
pub fn tr(key: &'static str) -> &'static str {
    tr_in(ui_language(), key)
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
        // 未命中 key 在任何语言下都回退返回 key 本身
        assert_eq!(tr_in(UiLanguage::Chinese, "不存在的文案"), "不存在的文案");
        assert_eq!(tr_in(UiLanguage::English, "不存在的文案"), "不存在的文案");
    }

    #[test]
    fn test_tr_in_english_lookup() {
        assert_eq!(tr_in(UiLanguage::English, "保存"), "Save");
        assert_eq!(tr_in(UiLanguage::English, "语言模式"), "Language Mode");
        // 中文直通
        assert_eq!(tr_in(UiLanguage::Chinese, "保存"), "保存");
    }
}
