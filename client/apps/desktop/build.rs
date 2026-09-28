fn main() {
    // tauri-build 只会为 tauri.conf.json / capabilities / permissions 声明 rerun-if-changed，
    // 并不会声明 bundle.icon 里那些图标文件（图标监听只在 tauri-cli 的 codegen 路径里有）。
    // 后果：只改动 icons/ 下的图标就 `pnpm build` 时，build script 不会重跑，
    // target/release/build/desktop-*/out/resource.lib 会沿用旧的那一份，
    // 于是 exe 里嵌的还是上一版图标（表现为「换了图标重新打包，图标没变」）。
    // 这里显式声明一遍，与 tauri.conf.json 的 bundle.icon 保持一致。
    for icon in [
        "icons/128x128.png",
        "icons/128x128@2x.png",
        "icons/icon.icns",
        "icons/icon.ico",
    ] {
        println!("cargo:rerun-if-changed={icon}");
    }

    // 防呆：Tauri v2 的 dev/prod 由 tauri crate 的 `custom-protocol` feature 决定
    // （见 tauri/build.rs：`let dev = !custom_protocol`）。直接用 `cargo build --release`
    // 编译时该 feature 没被启用，会编出「release 的壳 + dev 的芯」：启动后去连
    // tauri.conf.json 里的 devUrl（http://localhost:5174），表现为 ERR_CONNECTION_REFUSED。
    // tauri CLI 的 build 会自动启用该 feature，所以请固定用 `pnpm build` 打包。
    if std::env::var("PROFILE").unwrap_or_default() == "release" && tauri_build::is_dev() {
        println!(
            "cargo:warning=release 构建但 tauri 的 `custom-protocol` feature 未启用，\
             生成的 exe 启动后会去连接 devUrl（本地开发服务器）。请改用 `pnpm build`（tauri build）打包。"
        );
    }

    tauri_build::build()
}
