# zellij-pane-switcher

[English](README.md) | [简体中文](README_CN.md)

[![CI](https://github.com/pr-g/zellij-pane-switcher/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/pr-g/zellij-pane-switcher/actions/workflows/ci.yml)

一个面向 Zellij 的浮动 Pane 搜索与切换器。每个终端 Pane 单独展示为一行，
显示运行状态、类型图标、Tab-Pane ID、标题、简写目录和命令参数。

输入关键字即可模糊筛选，默认会自动选中最佳匹配；按 Enter 即可聚焦当前会话
或其他活动会话中最相关的结果。输入非空内容后按 `Ctrl+Enter`，会新建并切换
到一个 Tab，并用输入内容命名该 Tab 的默认 Pane。

```text
 FIND  › sr                                                     Ctrl+Enter new tab  Esc
──────────────────────────────────────────────────────────────────────────────
◓ ❉ 3-1  src renderer   src/ui           codex --full-auto
✓ 󰈙 3-2  editor notes   workspace/notes  nvim
✕ $ 4-3  old shell      tmp/elsewhere    zsh
↑↓/Tab move  Shift+Tab back  Enter/click jump  Ctrl+Enter new tab  wheel scroll  Esc close
```

## 功能

- 每个终端 Pane 一行，没有树形层级、折叠或详情面板。
- `Ctrl+Enter` 新建并切换到 Tab，用当前输入内容命名其默认 Pane。
- 行模板可配置，可显示标题、相对目录、状态、图标、会话、Tab、Tab ID、命令和 Pane ID。
- 浮动窗口宽高可配置为终端百分比或固定行列数。
- 命令参数优先完整显示，只有行宽不足时才省略。
- 搜索只匹配模板中可见的文字字段；模糊查询可以跨相邻字段匹配。
- 可跳转到当前会话或其他活动会话中的终端 Pane。
- 支持 UTF-8 安全匹配、高亮和 Unicode 显示宽度。
- 支持 Nerd Font 图标和纯文本图标。
- 状态和类型图标固定占一列；可用命令显示绿色对号，失败或退出命令显示红色叉号。
- 插件 Pane 标题固定显示为 `pane-switcher`，不再暴露完整 WASM 路径。
- 跟随 Zellij 当前主题颜色。
- 查询变化后自动选中最佳匹配；清空查询后回到当前 Pane，查询不变时的手动选择
  会跨 Zellij 数据刷新保持稳定。
- 支持嵌套覆盖模板片段和进度状态颜色。
- 搜索行、进度图标、类型图标、字段和选中背景使用不同的语义颜色。
- 鼠标悬浮行使用更柔和的全宽背景；悬浮不会改变键盘选择，和选中态同时命中时
  以更醒目的选中背景为准。
- 渲染只处理当前视口，不会为所有行生成终端输出。
- 搜索行缓存会复用于选择移动、重新渲染和动画帧。
- 鼠标左键单击可见 Pane 行即可立即聚焦；滚轮上下可移动当前选择。
- 鼠标悬浮只提供视觉反馈，不会改变 Enter 或点击的目标。

## 默认行

```text
{progress} {icon} {tab_id}-{pane_id}  {title}  {dir}  {command}
```

支持的模板字段：

| 字段         | 含义                             |
| :----------- | :------------------------------- |
| `{progress}` | 运行中、绿色对号、红色叉号或 `?` |
| `{icon}`     | Pane 或命令类型图标              |
| `{title}`    | Pane 标题，空时使用 `pane <id>`  |
| `{dir}`      | 相对 CWD，保留 `dir_depth` 级    |
| `{session}`  | 活动会话名称                     |
| `{tab}`      | Tab 名称                         |
| `{tab_id}`   | Zellij 稳定 Tab ID               |
| `{command}`  | 程序和完整命令参数               |
| `{pane_id}`  | 数字终端 Pane ID                 |

默认字段颜色：

| 字段                     | 颜色   |
| :----------------------- | :----- |
| `{title}`                | 青色   |
| `{dir}`                  | 蓝色   |
| `{session}`              | 洋红色 |
| `{tab}`                  | 橙色   |
| `{command}`              | 绿色   |
| `{tab_id}` / `{pane_id}` | 暗灰色 |

可以用统一的嵌套 `colors` 块覆盖模板片段和进度状态颜色：

```kdl
colors {
    "{title}" "red"
    "{tab_id}-{pane_id}" "yellow"
    progress {
        running "red"
        idle "green"
        exited "red"
        unknown "red"
    }
}
```

片段规则按完整模板部件匹配，因此 `"{tab_id}-{pane_id}"` 会连同中间的 `-`
一起着色。重叠规则以最长匹配优先，长度相同时后配置的规则优先。支持的主题色名：
`default`、`gray`、`green`、`cyan`、`magenta`、`orange`、`blue`、`red`、
`yellow`、`dark-gray`、`border`。无效颜色只回退对应项。

搜索命中的字符使用粗体黄色。悬浮行使用柔和背景，选中行使用更醒目的完整背景，
两者都保留字段颜色；同一行同时悬浮和选中时显示选中态。

只有当前模板中的文字字段会参与搜索。例如去掉 `{dir}` 后，目录就不会再被匹配。
目录可以简写显示，完整命令仍然参与搜索。
跨字段搜索会保留模板中的分隔符，因此默认格式可以直接搜索 `3-7`。
`{{` 和 `}}` 可以输出字面量花括号；未知或未闭合的模板字段会按普通文字处理。

## 安装与 Zellij 配置

推荐的用法是自动跟随最新 GitHub Release：

```kdl
plugins {
    pane-switcher location="https://github.com/pr-g/zellij-pane-switcher/releases/latest/download/pane-switcher.wasm" {
        scope "all"
        row_format "{progress} {icon} {tab_id}-{pane_id}  {title}  {dir}  {command}"
        width "72%"
        height "68%"
        dir_depth "2"
        colors {
            "{title}" "red"
            "{tab_id}-{pane_id}" "yellow"
            progress {
                running "red"
                idle "green"
                exited "red"
                unknown "red"
            }
        }
        icons "nerd"
        search_mode "fuzzy"
        spinner_fps "4"
    }
}

keybinds {
    shared_except "locked" {
        bind "Ctrl Shift e" {
            SwitchToMode "normal"
            LaunchOrFocusPlugin "pane-switcher" {
                floating true
                move_to_focused_tab true
            }
        }
    }

    session {
        bind "g" {
            LaunchOrFocusPlugin "pane-switcher" {
                floating true
                move_to_focused_tab true
            }
            SwitchToMode "normal"
        }
    }
}
```

`releases/latest/download` 会指向最新正式 Release。推荐的更新策略是在新
Zellij server 启动前运行一个轻量检查器：读取 GitHub 最新 Release 的资产摘要，
删除 Zellij 缓存目录中不再对应的数字命名插件文件。插件首次打开时会下载当前
版本，之后继续复用磁盘缓存，不需要每次重新编译。

使用这种启动前检查策略时不需要 `skip_plugin_cache`。如果没有启动器检查，
可以改用 `skip_plugin_cache true` 作为纯 Zellij 原生方案，但每次新打开插件都会
重新下载和编译。需要固定版本时则使用明确版本 URL。

已经打开的插件不会被热替换。要升级，先关闭插件，再按快捷键重新打开。

固定版本示例：

```text
https://github.com/pr-g/zellij-pane-switcher/releases/download/v0.1.8/pane-switcher.wasm
```

本地开发时也可以使用：

```text
file:/absolute/path/to/zellij-pane-switcher/dist/pane-switcher.wasm
```

## 操作

| 输入                        | 操作                    |
| :-------------------------- | :---------------------- |
| `Ctrl+Shift+e`              | 打开或聚焦 Pane 切换器  |
| `Ctrl+o`，然后 `g`          | 备用入口                |
| 可打印文字                  | 编辑搜索词              |
| Backspace                   | 删除一个字符            |
| `Ctrl+w`                    | 删除前一个词            |
| `Ctrl+u`                    | 清空搜索词              |
| Up / `Ctrl+p` / `Shift+Tab` | 选择上一条              |
| Down / `Ctrl+n` / `Tab`     | 选择下一条              |
| PageUp / PageDown           | 按十行移动              |
| Enter                       | 聚焦选中 Pane 并关闭    |
| `Ctrl+Enter`                | 新建 Tab 并用输入内容命名其 Pane |
| 左键单击 Pane 行            | 聚焦该 Pane 并关闭      |
| 鼠标悬浮 Pane 行            | 显示更柔和的行背景反馈  |
| 鼠标滚轮上 / 下             | 选择上一条 / 下一条     |
| Esc                         | 关闭，不跳转            |
| Left / Right                | 不执行操作              |

启动快捷键都是普通 Zellij KDL，可以按自己的环境修改。

## 配置项

```kdl
scope "all"                    // current | all
row_format "{progress} {icon} {tab_id}-{pane_id}  {title}  {dir}  {command}"
width "72%"                    // 终端宽度百分比
height "68%"                   // 终端高度百分比
dir_depth "2"                  // 目录显示级数
// width "120"                 // 固定 120 列
// height "40"                 // 固定 40 行
colors {
    "{title}" "red"
    "{tab_id}-{pane_id}" "yellow"
    progress {
        running "red"
        idle "green"
        exited "red"
        unknown "red"
    }
}
icons "nerd"                   // nerd | plain
search_mode "fuzzy"            // fuzzy | substring
spinner_fps "4"
follow_current_session "true"
```

- `scope`：只搜索当前会话，或搜索所有活动会话。
- `row_format`：控制显示顺序和搜索字段。
- `width` / `height`：支持百分比或固定行列数，默认 `72%` / `68%`。
- `dir_depth`：目录最多显示最后几级，默认 `2`。
- `command_tail`：仅为兼容旧配置保留，不再影响命令显示。
- `icons`：Nerd Font 图标或纯文本图标。
- `search_mode`：模糊匹配或连续子串匹配。
- `spinner_fps`：运行状态动画刷新频率。
- `follow_current_session`：初始选择当前聚焦 Pane。

未知配置项会被忽略，无效值会按字段回退。

## 自动构建与发布

GitHub Actions 会在 Pull Request 和 `main` 更新时执行：

- Rust 格式检查
- 完整测试
- 严格 Clippy
- WASM 构建
- Snapshot 渲染测试
- Zellij KDL 校验
- WASM 与 SHA-256 artifact 上传

推送 `v*` Tag 时，会自动创建或更新 GitHub Release，并发布：

```text
pane-switcher.wasm
pane-switcher.wasm.sha256
```

## 性能

预期使用规模是几十到几百个 Pane。

- 模板只在插件加载时解析一次。
- 只有查询、Pane 结构、相关描述数据或搜索字段变化时才重新构建搜索结果。
- 命令和目录更新只会让依赖它们的模板失效。
- 只渲染当前视口。
- 选择移动和动画帧复用缓存行。
- 命令和目录查询有有界重试，不会无限轮询。
- 10,000 行视口渲染有回归测试。

## 权限与限制

插件需要读取 Zellij 应用状态，并修改 Pane/会话焦点。

- 其他会话中的实时命令和目录可能不可用。
- 只列出可选中的活动终端 Pane。
- 插件 Pane、不可选择的 UI Pane 和无活动 Pane 的已关闭会话不会显示。
- Zellij 0.45.1 不支持插件自行注册全局快捷键，因此需要在 KDL 中配置。
- 插件获得权限后应用配置的浮动 Pane 宽高；百分比按当前终端尺寸计算。
- 支持输入法提交后的中文文本；拼音预编辑和候选窗口由终端与输入法管理。
- Zellij 0.45.1 的插件按键解码会把非 ASCII `Key::Char` 截断成 `u8`。
  发布构建会对 `zellij-utils` 应用一个兼容协议的最小补丁，确保单个汉字提交
  不会丢失。
- Zellij 负责原生浮动 Pane 边框，插件只渲染内容。

## 开发验证

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
./scripts/build-wasm.sh
./scripts/check-snapshot.sh
zellij --config examples/config.kdl setup --check
```

更详细的设计和研究说明见：

- [设计文档](docs/DESIGN.md)
- [技术研究](docs/RESEARCH.md)
