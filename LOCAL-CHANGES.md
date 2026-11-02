# 本地宠物版本

基于 Jotter v0.2.0（`901e1f8`）。悬浮小猫已替换为用户提供的 GUGUGAGA 素材。

## 素材与交互

- 素材副本：`ui/src/assets/pet/pet.json`、`spritesheet.webp`。
- 来源：仓库维护者提供的 `0648-endminguga` 素材文件夹；素材已包含在仓库中，构建无需访问原始文件夹。
- 帧表：1536 × 1872，8 列 × 9 行，单帧 192 × 208。
- 实现：`ui/src/components/Pet.svelte`，使用第一行待机、第七行展开表情，各播放前 6 帧。
- 沿用原有点击展开、拖动、角标与自动保存操作；窗口仍为 104 × 104。
- 动画短暂播放后停在首帧，系统开启减少动态效果时使用静态帧。
- 原 `Cat.svelte` 留在源码中作为原版参考，不会包含进最终构建。
- 浏览器预览补齐了事件释放的模拟接口，避免切换页面或热更新时报错。

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

生成 Windows EXE 推荐使用项目已有 GitHub Actions：

1. 打开 [本仓库 CI 工作流](https://github.com/XeonLeo99/jotter/actions/workflows/ci.yml)。
2. 如需启用 Actions，先启用工作流，再点击 `Run workflow`，选择 `main`。
3. 下载 `jotter-windows` 中的 `jotter.exe`（便携版）或 `*-setup.exe`（安装版）。
4. 也可下载容器构建任务的 `jotter-windows-portable`。

手动运行工作流不需要创建版本标签，也不会自动发布 Release；本机不用安装 MSVC、Rust 或 Docker。
现有工作流会执行检查并构建，GitHub Actions 的可用额度取决于账号与仓库设置。

原生 Windows 本地构建需要 Rust MSVC 工具链、Microsoft C++ Build Tools 和 Windows SDK。
项目另有 `make windows` 容器交叉编译方案，但需要已经能运行 Linux 容器的 Docker 环境。

## “技能”标签页

这是常用提示词/上下文片段库：保存名称和正文，支持搜索、编辑、删除、一键复制。
实现位于 `ui/src/components/Skills.svelte`，没有自动执行任务或调用 AI 的逻辑。
