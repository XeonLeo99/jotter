# 本地宠物版本

GUGUGAGA 宠物版 v0.3.1，基于 Jotter v0.2.0（`901e1f8`）。悬浮小猫已替换为用户提供的 GUGUGAGA 素材。

成品下载：[最新 Release](https://github.com/XeonLeo99/jotter/releases/latest)，选择 `*-portable.exe` 免安装运行。

## 素材与交互

- 素材副本：`ui/src/assets/pet/pet.json`、`spritesheet.webp`。
- 来源：仓库维护者提供的 `0648-endminguga` 素材文件夹；素材已包含在仓库中，构建无需访问原始文件夹。
- 帧表：1536 × 1872，8 列 × 9 行，单帧 192 × 208。
- 实现：`ui/src/components/Pet.svelte`，使用第一行待机、第七行展开表情，各播放前 6 帧。
- 沿用原有点击展开、拖动、角标与自动保存操作；窗口仍为 104 × 104。
- 动画短暂播放后停在首帧，系统开启减少动态效果时使用静态帧。
- 原 `Cat.svelte` 留在源码中作为原版参考，不会包含进最终构建。
- 浏览器预览补齐了事件释放的模拟接口，避免切换页面或热更新时报错。

## 截止、提醒与循环

点击任务右侧的日历按钮，设置本期截止日期和时间。提醒可选到期时、提前 10/30/60 分钟、一天或自定义整数分钟（最多一年）；也可关闭通知。

- 时间按当前系统本地时区解释，持久化为时间戳。日历循环保持本地钟点；夏令时不存在的时间向后调整到有效分钟，重复钟点选较早的一次。
- 到达提醒时间范围或逾期后，未完成任务让角标变红，并在任务下方显示“即将到期／已逾期”；不设提醒时在截止时变红。角标数字仍是所有卡片的未完成总数。全部紧急任务完成后恢复原色。
- 循环与 DDL 可分别设置，例如本期周五 18:00 截止、下周一 09:00 重置。重置时间必须晚于当前时间和本期截止时间。
- 完成后保持勾选，到下一周期恢复未完成，并同步顺延 DDL。每月保留原始日期：1 月 31 日 → 2 月月底 → 3 月 31 日。无 DDL 的任务也能循环。
- 错过多个周期只推进到当前周期，不生成一堆副本，也不保留历次完成记录。清理已完成任务会保留循环任务；明确删除该任务才停止循环。
- 原生后台每 15 秒检查一次，收起窗口后继续。退出／关机期间不会提醒，重新启动时补发当前仍需处理的提醒，每个截止时间成功提交后不重复。操作系统禁止通知时显示错误并隔一分钟重试。
- Windows 11 通知使用 WinRT，按当前用户注册 `HKCU\Software\Classes\AppUserModelId\com.ztcools.jotter`，便携版也可使用，不需要管理员权限。设置页可发送测试通知。勿扰模式可能仅收录到通知中心，系统接受通知不代表用户已经看到。
- 更新前退出旧程序；继续使用原有笔记数据，旧任务没有 DDL、提醒或循环。Markdown 导出包含截止、提醒及下次重置信息。
- 当前版本尚不提供通知内完成按钮、退出后定时唤醒或跨设备同步。

实现参考：[RudeSync 的提醒与补偿规则](https://github.com/Rudevin17/RudeSync/blob/main/docs/PRODUCT_SPEC.md)、[Super Productivity 的循环设置](https://github.com/super-productivity/super-productivity/wiki/2.06-Manage-Repeating-Tasks)、[Tauri WinRT 通知封装](https://github.com/tauri-apps/winrt-notification)。本项目按自己的 JSON 数据结构实现；日历运算使用 Chrono，Windows 接口使用 windows-rs。

## 开机自启

点击宠物展开面板，在底部「设置」页开启「开机自启」。默认关闭，只在当前用户登录 Windows 后启动，先显示宠物，便签面板保持收起。关闭开关即可取消。

- 只写入当前用户 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` 的 `Jotter` 项，无需管理员权限。
- 直接读取系统启动项，开关状态在重启后保留；不会因打开程序而自动注册自启。
- 启动路径加双引号，支持中文和空格；若便携 EXE 移动或改名，请从新位置运行后重新开启。删除或卸载前请先关闭。
- 如果在任务管理器中禁用了 Jotter，请在那里重新启用。
- 不修改笔记数据。注册表读写失败会显示错误，开关不会假装保存成功。
- Rust 测试只使用临时测试键，覆盖读取、写入、关闭、路径更新及其他项目不受影响，不操作真实启动项。

## 没有本地 MSVC 时

修改界面、检查类型、生成网页资源无需 MSVC。需要 Node.js 20 或以上：

```powershell
npx.cmd --yes pnpm@10.12.1 install --frozen-lockfile
npx.cmd --yes pnpm@10.12.1 check
npx.cmd --yes pnpm@10.12.1 build
npx.cmd --yes pnpm@10.12.1 dev
```

浏览器预览：`http://127.0.0.1:1420/preview.html?state=ball`；去掉查询参数预览笔记面板。
预览使用模拟数据，不具备真实桌面置顶、拖动和磁盘保存能力。

推送到 `main` 会自动通过 GitHub Actions 检查并构建 Windows EXE，也可手动触发：

1. 打开 [本仓库 CI 工作流](https://github.com/XeonLeo99/jotter/actions/workflows/ci.yml)。
2. 如需启用 Actions，先启用工作流，再点击 `Run workflow`，选择 `main`。
3. 下载 `jotter-windows` 中的 `jotter.exe`（便携版）或 `*-setup.exe`（安装版）。
4. 也可下载容器构建任务的 `jotter-windows-portable`。

推送 `main` 或手动运行工作流都不需要创建版本标签，也不会自动发布 Release；本机不用安装 MSVC、Rust 或 Docker。
现有工作流会执行检查并构建，GitHub Actions 的可用额度取决于账号与仓库设置。

## 发布 Release

更新 `package.json`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml` 和 `src-tauri/Cargo.lock` 中的应用版本，并更新 `.github/release-notes.md`。提交后推送相同版本的标签，例如：

```powershell
git tag v0.3.1
git push origin main v0.3.1
```

CI 检查和 Windows EXE 启动验收通过后，会自动创建 GitHub Release，上传便携 EXE 和安装包。已发布的版本使用新的版本号发布后续修改。

原生 Windows 本地构建需要 Rust MSVC 工具链、Microsoft C++ Build Tools 和 Windows SDK。
项目另有 `make windows` 容器交叉编译方案，但需要已经能运行 Linux 容器的 Docker 环境。

## “技能”标签页

这是常用提示词/上下文片段库：保存名称和正文，支持搜索、编辑、删除、一键复制。
实现位于 `ui/src/components/Skills.svelte`，没有自动执行任务或调用 AI 的逻辑。
