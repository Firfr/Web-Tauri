#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// 定义标准库中需要的类型
use std::{
    // 将Infallible类型导入，用于错误处理
    convert::Infallible,
    // 导入网络相关的IP地址类型
    net::{IpAddr, Ipv4Addr, SocketAddr},
    // 导入路径处理类型
    path::PathBuf,
    // 导入环境变量和命令行参数
    env,
};
// 导入Tauri的Manager trait
use tauri::Manager;

// 导入Hyper库相关组件用于HTTP服务器
use hyper::{
    // 导入HTTP连接相关组件
    server::conn::http1,
    // 导入服务函数
    service::service_fn,
    // 导入Request、Response和状态码
    Request, Response, StatusCode,
};
// 导入HTTP消息体工具
use http_body_util::Full;
// 导入字节操作
use bytes::Bytes;
// 导入TCP监听器
use tokio::net::TcpListener;
// 导入随机数生成
use rand::Rng;

// 主异步函数入口点
#[tokio::main]
// 定义main函数，返回Result类型
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 解析命令行参数获取端口
    let specified_port = parse_port_from_args();
    
    // 启动静态文件服务器部分开始
    // 如果指定了端口，先尝试使用指定端口，否则使用随机端口
    let (addr, listener) = if let Some(port) = specified_port {
        match bind_to_specific_port(port).await {
            Ok(result) => {
                println!("✅ 使用指定端口: {}", port);
                result
            },
            Err(e) => {
                eprintln!("⚠️  指定端口 {} 不可用: {}", port, e);
                eprintln!("🔄 切换到随机端口...");
                find_available_random_port().await?
            }
        }
    } else {
        println!("🔄 未指定端口，使用随机端口...");
        find_available_random_port().await?
    };

    // 获取可执行文件路径并构造静态资源根目录
    let exe_path = std::env::current_exe()
        .expect("无法获取程序路径");
    
    // 构造静态文件根目录：exe所在目录下的"代码"文件夹
    let static_root = exe_path
        .parent()
        .map(|p| p.join("代码"))
        .expect("无法计算静态文件目录");

    // 保存端口号，以便在setup闭包中使用
    let port = addr.port();

    // 启动HTTP服务器作为后台任务
    let static_root_clone = static_root.clone();
    // 创建异步任务来运行HTTP服务器
    tokio::spawn(async move {
        // 仅在调试模式下输出服务器启动信息
        #[cfg(debug_assertions)]
        println!("🚀 HTTP服务器已启动在 http://{}", addr);
        
        // 循环接受新连接
        loop {
            // 接受新的TCP连接
            let (stream, _) = listener.accept().await.unwrap();
            // 包装流以用于Hyper
            let io = hyper_util::rt::TokioIo::new(stream);
            
            // 克隆静态根目录以供任务使用
            let static_root = static_root_clone.clone();
            
            // 为每个连接创建一个异步任务
            tokio::task::spawn(async move {
                // 使用HTTP/1.1协议处理连接
                if let Err(_err) = http1::Builder::new()
                    .serve_connection(io, service_fn(move |req| serve_static(req, static_root.clone())))
                    .with_upgrades()
                    .await
                {
                    // 仅在调试模式下输出错误信息
                    #[cfg(debug_assertions)]
                    eprintln!("连接错误: {:?}", _err);
                }
            });
        }
    });

    // 创建Tauri应用部分开始
    // 初始化Tauri构建器
    tauri::Builder::default()
        // 设置函数，在应用初始化时执行，使用move关键字以获取所有权
        .setup(move |app| {
            // 获取预定义的主窗口
            let window = app.get_webview_window("main").expect("无法获取主窗口");
            // 导航到本地HTTP服务器，使用找到的随机端口
            let _ = window.navigate(format!("http://127.0.0.1:{}", port).parse().unwrap());
            // 设置窗口标题，包含端口号
            let _ = window.set_title(&format!("Web-Tauri-秦曱凧 | {}", port));
            // 返回成功结果
            Ok(())
        })
        // 运行应用，传入配置上下文
        .run(tauri::generate_context!())
        // 如果运行失败，输出错误信息
        .expect("❌ 应用启动失败");

    // 返回成功结果
    Ok(())
}

// 查找可用随机端口的辅助函数
async fn find_available_random_port() -> Result<(SocketAddr, TcpListener), Box<dyn std::error::Error + Send + Sync>> {
    let mut rng = rand::thread_rng();
    
    // 尝试最多10次找到可用端口
    for _ in 0..10 {
        // 生成大于1024的随机端口（范围：1025-65535）
        let port = rng.gen_range(1025..=65535);
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
        
        match TcpListener::bind(&addr).await {
            Ok(listener) => {
                return Ok((addr, listener));
            }
            Err(_) => continue, // 端口不可用，尝试下一个
        }
    }
    
    // 如果随机尝试失败，回退到顺序查找
    for port in 1025..=65535 {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
        match TcpListener::bind(&addr).await {
            Ok(listener) => {
                return Ok((addr, listener));
            }
            Err(_) => continue, // 端口不可用，尝试下一个
        }
    }
    
    Err("无法找到可用端口".into())
}

// 静态文件服务函数，处理HTTP请求
async fn serve_static(
    // HTTP请求参数
    req: Request<hyper::body::Incoming>,
    // 静态文件根目录路径
    base_path: PathBuf,
) -> Result<Response<Full<Bytes>>, Infallible> {
    // 获取请求的URI路径
    let url_path = req.uri().path();
    
    // 调试信息：输出请求的路径
    #[cfg(debug_assertions)]
    println!("请求路径: {}", url_path);
    
    // 处理根路径并清理URL路径
    let clean_path = if url_path == "/" {
        // 如果是根路径，则映射到index.html
        "index.html".to_string()
    } else {
        // 否则移除路径开头的斜杠
        url_path.trim_start_matches('/').to_string()
    };

    // 构造完整文件路径
    let file_path = base_path.join(&clean_path);

    // 调试信息：输出实际文件路径
    #[cfg(debug_assertions)]
    println!("实际文件路径: {:?}", file_path);

    // 检查路径是否在基础目录内，防止路径穿越攻击
    if !file_path.starts_with(&base_path) {
        // 如果路径不在基础目录内，返回禁止访问响应
        return forbidden_response();
    }

    // 检查文件是否存在
    if !file_path.exists() {
        // 如果文件不存在，返回404响应
        return not_found_response();
    }

    // 读取文件内容
    match tokio::fs::read(&file_path).await {
        // 成功读取文件
        Ok(contents) => {
            // 将内容包装为Full<Bytes>类型
            let body = Full::from(contents);
            // 根据文件扩展名确定内容类型
            let content_type = match file_path.extension().and_then(|ext| ext.to_str()) {
                // HTML文件类型
                Some("html") | Some("htm") => "text/html",
                // CSS样式文件类型
                Some("css") => "text/css",
                // JavaScript文件类型
                Some("js") => "application/javascript",
                // JSON文件类型
                Some("json") => "application/json",
                // PNG图片类型
                Some("png") => "image/png",
                // JPEG图片类型
                Some("jpg") | Some("jpeg") => "image/jpeg",
                // GIF图片类型
                Some("gif") => "image/gif",
                // SVG图片类型
                Some("svg") => "image/svg+xml",
                // ICO图标类型
                Some("ico") => "image/x-icon",
                // WOFF字体类型
                Some("woff") => "font/woff",
                // WOFF2字体类型
                Some("woff2") => "font/woff2",
                // TTF字体类型
                Some("ttf") => "font/ttf",
                // EOT字体类型
                Some("eot") => "application/vnd.ms-fontobject",
                // 默认文本类型
                _ => "text/plain",
            };
            
            // 调试信息：输出成功返回的文件及类型
            #[cfg(debug_assertions)]
            println!("成功返回文件: {:?}, 类型: {}", file_path, content_type);
            
            // 构造成功响应，添加缓存控制头
            Ok(Response::builder()
                // 设置状态码为200 OK
                .status(StatusCode::OK)
                // 设置内容类型头部
                .header("Content-Type", content_type)
                // 设置CORS头部允许所有来源
                .header("Access-Control-Allow-Origin", "*")
                // 添加缓存控制头，防止浏览器缓存
                .header("Cache-Control", "no-cache, no-store, must-revalidate")
                .header("Pragma", "no-cache")
                .header("Expires", "0")
                // 设置响应体
                .body(body)
                // 解包结果
                .unwrap())
        }
        // 读取文件失败
        Err(_) => not_found_response(),
    }
}

// 返回404未找到响应的辅助函数
fn not_found_response() -> Result<Response<Full<Bytes>>, Infallible> {
    // 调试信息：输出返回404
    #[cfg(debug_assertions)]
    println!("返回 404");
    // 构造404响应
    Ok(Response::builder()
        // 设置状态码为404
        .status(StatusCode::NOT_FOUND)
        // 设置内容类型为纯文本
        .header("Content-Type", "text/plain")
        // 设置缓存控制头
        .header("Cache-Control", "no-cache, no-store, must-revalidate")
        .header("Pragma", "no-cache")
        .header("Expires", "0")
        // 设置响应体内容
        .body(Full::from("404 Not Found"))
        // 解包结果
        .unwrap())
}

// 返回403禁止访问响应的辅助函数
fn forbidden_response() -> Result<Response<Full<Bytes>>, Infallible> {
    // 调试信息：输出返回403
    #[cfg(debug_assertions)]
    println!("返回 403");
    // 构造403响应
    Ok(Response::builder()
        // 设置状态码为403
        .status(StatusCode::FORBIDDEN)
        // 设置内容类型为纯文本
        .header("Content-Type", "text/plain")
        // 设置缓存控制头
        .header("Cache-Control", "no-cache, no-store, must-revalidate")
        .header("Pragma", "no-cache")
        .header("Expires", "0")
        // 设置响应体内容
        .body(Full::from("403 Forbidden"))
        // 解包结果
        .unwrap())
}

// 解析命令行参数获取端口
fn parse_port_from_args() -> Option<u16> {
    let args: Vec<String> = env::args().collect();
    
    // 遍历参数寻找端口参数
    for i in 1..args.len() {
        let arg = &args[i];
        
        // 支持多种参数格式：
        // --port 8080
        // -p 8080
        // --port=8080
        // -p=8080
        
        if arg == "--port" || arg == "-p" {
            if i + 1 < args.len() {
                if let Ok(port) = args[i + 1].parse::<u16>() {
                    return Some(port);
                }
            }
        } else if arg.starts_with("--port=") {
            let port_str = &arg[7..]; // 跳过 "--port=" 前缀
            if let Ok(port) = port_str.parse::<u16>() {
                return Some(port);
            }
        } else if arg.starts_with("-p=") {
            let port_str = &arg[3..]; // 跳过 "-p=" 前缀
            if let Ok(port) = port_str.parse::<u16>() {
                return Some(port);
            }
        }
    }
    
    None
}

// 绑定到指定端口的函数
async fn bind_to_specific_port(port: u16) -> Result<(SocketAddr, TcpListener), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
    let listener = TcpListener::bind(&addr).await?;
    Ok((addr, listener))
}
