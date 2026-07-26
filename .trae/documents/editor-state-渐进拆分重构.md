# EditorState 渐进式字段聚类重构计划

## 摘要（Summary）

将 `crates/aether-win32/src/editor/mod.rs` 中 150+ 字段的 god struct `EditorState` 按领域渐进拆分为若干子结构（LspState / RemoteState / FindState / TabBarState / HoverState / PrevFrameState / MousePressState / ContextMenusState）。

**策略（用户已确认）**：渐进式——只移动字段到子 struct，方法仍挂在 `EditorState` 上，通过 `self.lsp.client`、`state.find.query` 等方式访问。每个子 struct 一个独立步骤（独立提交），编译驱动修错，任何一步完成都是可停的健康状态。

**不在本次范围内**：抽离 AI/终端独立 crate、孤儿 crate（aether-plugin/aether-dap）处理、repowiki 文档同步。

## 现状分析（Current State）

- `EditorState` 定义于 [crates/aether-win32/src/editor/mod.rs:242](file:///d:/Application/牧羊人编辑器/crates/aether-win32/src/editor/mod.rs#L242)，约 150 个 pub 字段，混合窗口、渲染缓存、LSP、SSH/远程、查找替换、标签页、拖拽/hover 瞬态、脏追踪快照等。
- 方法分布在 14 个 `impl EditorState` 块中：editor/{mod,tabs,remote,dialogs,files,lsp,cursor,git,file_tree,events,editing,find,ime,ai}.rs。
- 外部访问方：render/*.rs、window/{window_messages,keyboard_handler,mouse_handler} 等，均通过 `EDITOR_STATE.with(...)` 拿到 `Rc<RefCell<EditorState>>` 后直接访问字段。
- 关键访问点统计（grep）：
  - LSP 相关字段（lsp_*/completion_*/tokio_runtime 等）：约 61 处，集中在 editor/lsp.rs（56 处）。
  - 远程相关字段：约 217 处，集中在 editor/remote.rs（96 处）、render/remote*.rs、window/ 各 handler。
  - 查找替换字段：约 113 处，集中在 editor/find.rs（70 处）。
- 已确认的小腐化点（顺手修）：`git: GitIntegration`（mod.rs:312）与 `git_panel: GitIntegration`（mod.rs:397）重复字段——本次**不动**（属于清理项，用户未选），留作后续。

## 子结构设计（Proposed Sub-structs）

所有子 struct 定义在 `crates/aether-win32/src/editor/mod.rs`（与 EditorState 同文件，避免移动类型定义造成大范围 import 变更），字段保持原有可见性（pub / pub(crate)），并 `#[derive(Default)]` 或在 `EditorState::new` 中原样初始化。

### 第 1 步：FindState（最小、最内聚，先做以验证流程）
- 字段：`find_visible, replace_visible, find_query, replace_text, find_results, find_active_index, find_focus, last_find_query, find_result_version`
- 访问方式：`self.find.query` / `state.find.results`（字段名去重前缀：`find_query`→`query`，`find_visible`→`visible` 等；`replace_*` 保留 replace 前缀）
- 影响面：editor/find.rs（70 处）、render/find.rs、render/mod.rs、window/keyboard_handler/*、window/mouse_handler/l_button_down/content_area.rs，约 113 处。

### 第 2 步：LspState
- 字段：`legacy_lsp_client, lsp_rx, lsp_runtime, tokio_runtime, lsp_client, lsp_diagnostics, completion_items, completion_visible, completion_selected, completion_trigger_line, completion_trigger_col, hover_content`
- 字段名简化：`lsp_client`→`client`，`lsp_diagnostics`→`diagnostics`，`lsp_rx`→`rx`，`lsp_runtime`→`legacy_runtime`（与 `tokio_runtime` 区分）；completion_* 保留原名。
- **注意**：`EditorState` 已有顶层字段 `diagnostics: HashMap<String, Vec<DiagnosticItem>>`（问题面板预留），与 LSP 的 `lsp_diagnostics: HashMap<Url, Vec<Diagnostic>>` 是两个不同东西；归入子 struct 后分别通过 `state.diagnostics`（顶层）和 `state.lsp.diagnostics` 访问，不冲突。
- 影响面：editor/lsp.rs（56 处）、editor/editing.rs、editor/files.rs，约 61 处。
- `bg_highlighter` / `hl_request_version`（tree-sitter，非 LSP）**留在顶层**，不进 LspState。

### 第 3 步：RemoteState
- 字段：`ssh_dialog, remote_session, remote_file_tree, selected_remote_node, hover_remote_node, remote_scroll_y, clone_dialog, ssh_manager_panel, active_ssh_index, ssh_connecting`
- 访问方式：`state.remote.session`、`state.remote.ssh_dialog` 等。
- 影响面：约 217 处，分布最广（editor/remote.rs 96 处、render/remote*.rs 32 处、window/ 多个 handler、editor/cursor.rs、editor/files.rs、render/mod.rs）。这是最大一步，单独一个提交。
- `git_cloning`、`git`、`git_panel` 属 Git 域，**不进** RemoteState，留顶层。

### 第 4 步：TabBarState
- 字段：`tabs, active_tab, tab_layouts, hover_tab, tab_scroll_x, plus_button_rect, plus_button_hover, dragging_tab, tab_drop_index, tab_drag_start, last_closed_tab`
- **注意**：`content: TabContent` 与 tabs 切换通过 swap 同步，耦合深，**留在顶层**不进 TabBarState；`last_active_tab` 属脏追踪快照，进 PrevFrameState（第 6 步）。
- 影响面：editor/tabs.rs、render/tabs.rs、window/ mouse/keyboard handler。

### 第 5 步：HoverState + MousePressState（两个小结构，同一提交）
- HoverState：`hover_tooltip, hover_last_mouse_x, hover_last_mouse_y`（`hover_content` 已在第 2 步进 LspState；`tooltip_state` 是独立部件留顶层；`hover_tab`/`hover_file_node`/`hover_remote_node`/`titlebar_hover_button` 等各处 hover 已分别归属标签/远程等域，不并入）。
- MousePressState：`lpress_start, lpress_x, lpress_y, lpress_target, lpress_index, lbutton_down`
- 影响面：window/mouse_handler/*、editor/mod.rs（compute_hover_tooltip_text / clear_hover_tooltip）。

### 第 6 步：PrevFrameState（脏追踪快照聚合）
- 字段：`last_cursor_line, last_cursor_col, last_scroll_y, last_selection_start, last_selection_end, last_sidebar_content, last_sidebar_visible, last_activity_bar_visible, last_right_panel_visible, last_bottom_panel_visible, last_status_message, last_active_tab`
- 访问方式：`state.prev.cursor_line`（去 `last_` 前缀）。
- 影响面：render/ 脏矩形判定逻辑、editor/events.rs。

### 第 7 步：ContextMenusState（小结构，收尾）
- 字段：`explorer_context_menu, file_node_context_menu, tab_context_menu, activity_bar_context_menu`
- `user_menu` 是菜单栏部件而非上下文菜单，留顶层。

### 最终留在 EditorState 顶层的字段
窗口/渲染核心（hwnd, d2d_factory, render_ctx, text_renderer, theme, content, cached_line_numbers, layout, dpi_scale, window_width/height）、全局 UI 部件（menu_bar, activity_bar, status_bar, command_palette, file_tree, settings_panel, tabs_panel, ai_panel, search_panel, terminal_panel, status_message, icons）、子系统服务（git, git_panel, ime, key_map, focus_manager, event_queue, dirty_tracker, inline_completion_service, bg_highlighter, app_settings, recent_projects, new_project_dialog）、以及各类零散标量（is_maximized, is_selecting, sidebar_scroll_y, fs_watch_until 等）。

## 实施规则（How）

每一步骤统一流程：
1. 在 `editor/mod.rs` 定义子 struct（字段从 EditorState 原样剪切，保留文档注释与可见性）。
2. `EditorState` 中替换为单个字段（如 `pub find: FindState`），`EditorState::new` 中对应初始化。
3. `cargo check -p aether-win32` 编译驱动修复所有访问点（`self.find_query` → `self.find.query`）。禁止借机改任何逻辑。
4. `cargo test -p aether-core -p aether-win32`（win32 仅 conpty_smoke）+ `cargo clippy -p aether-win32` 无新增警告。
5. 单独 commit（如 `refactor(win32): extract FindState from EditorState`）。

## 假设与决策（Assumptions & Decisions）

- 方法一律留在 `impl EditorState` 上，不下沉到子 struct（用户已确认的渐进策略）。
- 子 struct 与 EditorState 同文件定义，减少 import 扰动；后续"彻底式"重构再考虑拆文件。
- 字段重命名仅限"去冗余前缀"（find_*/lsp_*），其余保持原名，降低 diff 噪音。
- `content`、`git/git_panel`、双 runtime（`lsp_runtime` vs `tokio_runtime`）等深层问题本次不动。
- 每步一个 commit，步骤间可随时中止，仓库始终处于可编译状态。

## 验证（Verification）

1. 每步：`cargo check -p aether-win32` 通过。
2. 全部完成后：`cargo build -p aether-win32`、`cargo test --workspace`（至少 aether-core 全量 + win32 smoke）通过。
3. `cargo clippy -p aether-win32 -- -D warnings` 不引入新警告（基线：重构前先记录现有警告数）。
4. 手动冒烟：启动 `aether-app`，验证查找替换（Ctrl+F）、打开文件夹 + LSP 补全/hover、标签拖拽、远程对话框渲染、鼠标 hover tooltip 五类被拆功能行为无回归。
5. 结构验收：`EditorState` 顶层字段数从 ~150 降至 ~70，8 个子 struct 就位。
