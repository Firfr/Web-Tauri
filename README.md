# Web-Tauri

使用 Tauri 创建的软件，打开后会加载软件同目录下的`代码`目录中的网站代码，并运行。

软件的作用是将静态站点代码变成一个Windows上的可执行便携软件。

欢迎关注我B站账号 [秦曱凧](https://space.bilibili.com/17547201) (读作 qín yuē zhēng)  

如果这个项目有帮到你。欢迎start。也厚颜期待您的[打赏](https://gitee.com/firfe/me)。

## 使用方法

- 下载可执行程序到本地
- 在这个软件的目录中创建一个`代码`目录
- 将静态站点代码放入该目录中
- 静态代码必须包含`index.html`文件
- 双击运行软件

```
.
|-Web-Tauri-秦曱凧.exe
|-代码
| |-index.html
| |-其他文件
```

## 环境配置

- 系统：Windows 10 +
- Node.js
  - 版本：24
  - 官网：https://nodejs.org/zh-cn/download
  - 下载安装包安装即可
  - 设置 npm 源为国内源，方便下载依赖包
    ```bash
    npm config set registry https://registry.npmmirror.com
    ```
- Microsoft C++ 构建工具
  - Tauri 编译 Rust 后端需要 Visual Studio 的 C++ 工具链。
  - 官网：https://visualstudio.microsoft.com/zh-hans/visual-cpp-build-tools/
  - 下载生成工具，安装
  - 安装时勾选：
    - 使用 C++ 的桌面开发
    - 确保包含 Windows 10/11 SDK 和 CMake 工具（可选但推荐）
- Rust
  - 设置 Rust 镜像源（USTC 中国科大）
    - 中科大镜像源：https://mirrors.ustc.edu.cn/help/rust-static.html
    ```powershell
    $env:RUSTUP_DIST_SERVER="https://mirrors.ustc.edu.cn/rust-static"
    $env:RUSTUP_UPDATE_ROOT="https://mirrors.ustc.edu.cn/rust-static/rustup"
    ```
  - 下载 rustup-init.exe
    - 官网：https://rust-lang.org/zh-CN/tools/install/
  - 下载后正常安装即可
- 开发工具推荐 VSCode
  - 官网：https://code.visualstudio.com/Download
  - 建议安装 `System Installer` 版本
- Tauri
  - 官网：https://tauri.app/zh-cn/
  - 版本 2.9.5
  - 对应的 `cli` 版本为 `2.9.6`
  - 对应的项目脚手架工具 `tauri-app` 版本 `4.6.2`
- 个人习惯锁定软件具体版本，防止不必要的意外

## 创建项目

> 创建新的项目或拉取本仓库  
> 拉取本仓库时，手动在项目根目录中创建一个 `代码` 目录，  
> 把静态站点代码放入该目录中。  
> 必须包含`index.html`文件，静态站点代码，随便写一个就行。

- 命令行转到项目父目录
- 创建项目
  ```bash
  npm create tauri-app@4.6.2 web-tauri
  ```
- 选项
  - `? Identifier (com.win.web-tauri) ›` 直接回车
    - 应用的唯一 Bundle ID（包标识符）
  - 选择应用前端（UI 层）用什么技术栈
    ```
    ? Choose which language to use for your frontend ›
    ❯ TypeScript / JavaScript  (pnpm, yarn, npm, deno, bun)
      Rust
      .NET
    ```
    - 选择 `TypeScript / JavaScript`
  - 选择前端项目的包管理器，
    ```
    ? Choose your package manager ›
    ❯ npm
      pnpm
      yarn
      deno
      bun
    ```
    - 选择 `npm`
  - 选择前端 UI 的开发模板（框架）
    ```
    ? Choose your UI template ›
    ❯ Vanilla  无任何框架
      Vue
      Svelte
      React
      Solid
      Angular
      Preact
    ```
    - 选择 `Vanilla`，即无任何框架
  - 选择前端逻辑语言
    ```
    ? Choose your UI flavor ›
      TypeScript
    ❯ JavaScript
    ```
    - 选择 `JavaScript`
- 创建完成后，进入项目目录 `web-tauri`
- 修改文件
  - 参考本项目文件内容修改，所有需要修改的文件基本做到了一行一注释
  - 把 `src` 目录重命名为 `代码`
    - 前端端静态资源存放位置
  - 修改`src-tauri/tauri.conf.json`
    > Tauri特定的配置文件，定义了：
    - 应用程序元数据（名称、版本等）
    - 窗口配置
    - 构建和打包选项
  - 修改`src-tauri/Cargo.toml`
    > Rust项目的依赖管理和配置文件，定义了：
    - 项目基本信息
    - 依赖库列表（如tauri、tokio、hyper等）
    - 构建配置
  - 修改`src-tauri/src/main.rs`
    > 这是应用程序的主要入口点，包含：
    - HTTP服务器的实现
    - 静态文件服务功能
    - Tauri应用的初始化和配置

## 运行&编译

- 安装依赖
  ```bash
  npm install
  ```
- 调试运行
  - 把 `./代码` 目录复制到 `./src-tauri/target/debug` 下
    - 目录不存在则手动创建
  - 调试运行
    ```bash
    npm run tauri dev
    ```
- 编译安装包
  ```bash
  npm run tauri build
  ```
  - 编译后安装包在 `./src-tauri/target/release` 目录下
  - 只有后缀为 `.exe` 的文件是所需要的，其他的不用管

> 遇到报错等，把报错发给AI，让AI解决，记住要给出运行环境的具体版本，要不然AI的回答可能会是旧版本的。
