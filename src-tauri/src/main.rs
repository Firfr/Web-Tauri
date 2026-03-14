#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// 定义标准库中需要的类型
use std::{
  convert::Infallible,                 // 用于错误处理的类型
  env,                                 // 访问环境变量和命令行参数
  fs,                                  // 文件系统操作（如果实际使用了）
  net::{IpAddr, Ipv4Addr, SocketAddr}, // IPv4 地址和网络套接字类型
  path::{Path, PathBuf},               // 文件路径处理（Path 为借用，PathBuf 为拥有）
};
// 导入 Tauri 的核心组件：WebviewUrl 和 WebviewWindowBuilder
use tauri::{WebviewUrl, WebviewWindowBuilder};

// 导入Hyper库相关组件用于HTTP服务器
use hyper::{
  // 导入HTTP连接相关组件
  server::conn::http1,
  // 导入服务函数
  service::service_fn,
  // 导入Request、Response和状态码
  Request,
  Response,
  StatusCode,
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
  #[cfg(debug_assertions)]
  println!("获取可执行文件所在路径");

  let exe_path = std::env::current_exe().expect("无法获取程序路径");
  // 获取可执行文件所在目录（只解包一次）
  let exe_dir = exe_path.parent().expect("无法获取可执行文件所在目录");

  #[cfg(debug_assertions)]
  println!("可执行文件所在路径: {:?}", exe_path);

  #[cfg(debug_assertions)]
  println!("确保可执行文件所在目录下有 \"代码\" 目录");

  let static_root = exe_dir.join("代码");
  // 检查静态文件目录是否存在
  if !static_root.exists() {
    panic!("❌ 错误：静态文件目录 \"代码\" 不存在：{:?}", static_root);
  }
  if !static_root.is_dir() {
    panic!("❌ 错误：\"代码\" 不是一个目录：{:?}", static_root);
  }

  #[cfg(debug_assertions)]
  println!("静态文件目录 \"代码\" 目录下存在文件 index.html");
  let index_path = static_root.join("index.html");
  if !index_path.exists() {
    panic!("❌ 错误：index.html 文件不存在 {:?}", static_root);
  }

  #[cfg(debug_assertions)]
  println!("从配置文件读取窗口标题");

  let config_path = exe_dir.join("配置.json");
  let title = read_title(&config_path);

  #[cfg(debug_assertions)]
  println!("获取端口：命令行参数 > 配置文件 > 随机生成");
  let final_port = get_port_from_args_or_config_or_random(&config_path).await?;

  // 启动静态文件服务器部分开始
  // 使用最终确定的端口
  let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), final_port);
  let listener = TcpListener::bind(&addr).await?;
  println!("✅ 使用端口：{}", final_port);

  // 保存端口号，以便在 setup 闭包中使用
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
          .serve_connection(
            io,
            service_fn(move |req| serve_static(req, static_root.clone())),
          )
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

  // 拼接数据目录路径
  let data_dir = exe_dir.join("数据");

  #[cfg(debug_assertions)]
  println!("🔍 查找自定义图标...");

  let icon_extensions = ["png", "ico", "jpg", "jpeg"];
  let mut icon_path_str: Option<String> = None;

  for (_index, ext) in icon_extensions.iter().enumerate() {
    let candidate = data_dir.join(format!("图标.{}", ext));

    #[cfg(debug_assertions)]
    println!("尝试 #{}: {}", _index + 1, candidate.display());

    if candidate.exists() {
      #[cfg(debug_assertions)]
      println!("✅ 发现图标文件：{:?} (格式：{})", candidate, ext);
      icon_path_str = Some(candidate.to_string_lossy().to_string());
      break;
    } else {
      #[cfg(debug_assertions)]
      println!("⏭️  未找到：{:?}", candidate);
    }
  }

  let icon_path_str = match icon_path_str {
    Some(path) => {
      #[cfg(debug_assertions)]
      println!("🎨 将使用自定义图标：{}", path);
      path
    }
    None => {
      #[cfg(debug_assertions)]
      println!("ℹ️  未找到自定义图标，使用默认图标");
      String::new()
    }
  };

  #[cfg(debug_assertions)]
  println!("设置 WebView2 数据目录");

  #[cfg(debug_assertions)]
  println!("数据目录路径：{:?}", data_dir);

  if let Err(_e) = fs::create_dir_all(&data_dir) {
    #[cfg(debug_assertions)]
    {
      eprintln!("❌ 创建数据目录失败：{:?}", _e);
      eprintln!("      无法创建数据目录：{:?}", _e);
    }
  } else {
    #[cfg(debug_assertions)]
    println!("✅ 数据目录创建成功");
    env::set_var("WEBVIEW2_USER_DATA_FOLDER", data_dir.to_str().unwrap());
    #[cfg(debug_assertions)]
    println!("环境变量已设置：{}", data_dir.to_str().unwrap());
  }

  // 创建Tauri应用部分开始
  // 初始化Tauri构建器
  tauri::Builder::default()
    // 设置函数，在应用初始化时执行，使用move关键字以获取所有权
    .setup(move |app| {
      let parsed_url = format!("http://127.0.0.1:{}", port).parse().unwrap();
      #[cfg(debug_assertions)]
      println!("当前项目使用的网址：{}", parsed_url);

      #[cfg(debug_assertions)]
      println!("创建窗口并设置大小标题");
      let mut _window = WebviewWindowBuilder::new(app, "main", WebviewUrl::External(parsed_url))
        .title(format!("{} | {}", title, port))
        .inner_size(1200.0, 900.0);

      if !icon_path_str.is_empty() && Path::new(&icon_path_str).exists() {
        #[cfg(debug_assertions)]
        println!("加载自定义图标：{}", icon_path_str);

        match load_icon_from_file(&icon_path_str) {
          Ok(icon) => {
            #[cfg(debug_assertions)]
            println!("图标加载成功");
            _window = _window.icon(icon).expect("设置图标失败");
          }
          Err(_e) => {
            #[cfg(debug_assertions)]
            eprintln!("图标加载失败：{:?}", _e);
          }
        }
      } else {
        #[cfg(debug_assertions)]
        println!("使用默认图标");
      }

      let _window = _window.build()?;

      Ok(())
    })
    // 运行应用，传入配置上下文
    .run(tauri::generate_context!())
    // 如果运行失败，输出错误信息
    .expect("❌ 应用启动失败");

  // 返回成功结果
  Ok(())
}

// 从配置文件读取标题
fn read_title(config_path: &PathBuf) -> String {
  #[cfg(debug_assertions)]
  println!("尝试读取配置文件获取标题：{:?}", config_path);

  // 默认标题
  let default_title = "Web-Tauri-秦曱凧";

  // 读取配置文件内容
  let config_content = match std::fs::read_to_string(config_path) {
    Ok(content) => {
      #[cfg(debug_assertions)]
      println!("✅ 配置文件读取成功");
      content
    }
    Err(_e) => {
      #[cfg(debug_assertions)]
      {
        eprintln!("⚠️  配置文件不存在或读取失败：{}", _e);
        eprintln!("🔄 将使用默认标题继续...");
      }
      return default_title.to_string();
    }
  };

  // 解析 JSON 内容
  let config_value: serde_json::Value = match serde_json::from_str(&config_content) {
    Ok(value) => value,
    Err(_e) => {
      #[cfg(debug_assertions)]
      {
        eprintln!("⚠️  配置文件 JSON 格式错误：{}", _e);
        eprintln!("🔄 将使用默认标题继续...");
      }
      return default_title.to_string();
    }
  };

  // 读取标题字段
  match config_value.get("标题").and_then(|v| v.as_str()) {
    Some(t) if !t.trim().is_empty() => {
      #[cfg(debug_assertions)]
      println!("✅ 从配置文件读取标题：{}", t);
      t.to_string()
    }
    _ => {
      #[cfg(debug_assertions)]
      {
        eprintln!("⚠️  配置文件中 \"标题\" 字段不存在或为空");
        eprintln!("🔄 使用默认标题：{}", default_title);
      }
      default_title.to_string()
    }
  }
}

// 从命令行参数、配置文件或随机生成获取端口的函数
async fn get_port_from_args_or_config_or_random(
  config_path: &PathBuf,
) -> Result<u16, Box<dyn std::error::Error + Send + Sync>> {
  // 尝试从命令行参数获取端口
  if let Some(port) = get_port_form_args() {
    #[cfg(debug_assertions)]
    println!("✅ 从命令行参数获取端口：{}", port);
    return Ok(port);
  }

  // 尝试从配置文件获取端口
  if let Some(port) = read_config_port(config_path) {
    #[cfg(debug_assertions)]
    println!("✅ 从配置文件获取端口：{}", port);
    return Ok(port);
  }

  // 生成随机端口
  let random_port = find_available_random_port().await?.0.port();
  #[cfg(debug_assertions)]
  println!("🔄 使用随机生成的端口：{}", random_port);
  Ok(random_port)
}

// 解析命令行参数获取端口
fn get_port_form_args() -> Option<u16> {
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

// 从配置文件读取端口
fn read_config_port(config_path: &PathBuf) -> Option<u16> {
  // 读取配置文件内容
  let config_content = match std::fs::read_to_string(config_path) {
    Ok(content) => content,
    Err(_) => {
      #[cfg(debug_assertions)]
      println!("ℹ️  配置文件不存在，跳过端口读取");
      return None;
    }
  };

  // 解析 JSON 内容
  let config_value: serde_json::Value = match serde_json::from_str(&config_content) {
    Ok(value) => value,
    Err(_) => {
      #[cfg(debug_assertions)]
      println!("ℹ️  配置文件 JSON 格式错误，跳过端口读取");
      return None;
    }
  };

  // 读取端口字段并验证
  match config_value.get("端口").and_then(|v| v.as_u64()) {
    Some(p) if p > 1024 && p <= 65535 => {
      #[cfg(debug_assertions)]
      println!("✅ 从配置文件读取有效端口：{}", p);
      Some(p as u16)
    }
    Some(p) => {
      eprintln!(
        "⚠️  配置的端口 {} 无效（必须大于 1024 且小于等于 65535）",
        p
      );
      None
    }
    None => {
      #[cfg(debug_assertions)]
      println!("ℹ️  配置文件中不存在 \"端口\" 字段");
      None
    }
  }
}

// 查找可用端口
async fn find_available_random_port(
) -> Result<(SocketAddr, TcpListener), Box<dyn std::error::Error + Send + Sync>> {
  let mut rng = rand::thread_rng();

  // 尝试最多10次找到可用端口
  #[cfg(debug_assertions)]
  println!("🔄 随机生成的端口，尝试最多10次");
  for _ in 0..10 {
    // 生成大于1024的随机端口（范围：1025-65535）
    let port = rng.gen_range(1025..=65535);
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);

    match TcpListener::bind(&addr).await {
      Ok(listener) => {
        #[cfg(debug_assertions)]
        println!("🔄 尝试绑定使用端口 {} ", port);
        return Ok((addr, listener));
      }
      Err(_) => {
        #[cfg(debug_assertions)]
        println!("🔄 端口 {} 不可用，在次随机生成一个", port);
        continue;
      }
    }
  }

  // 如果随机尝试失败，回退到顺序查找
  #[cfg(debug_assertions)]
  println!("🔄 随机尝试失败，到顺序查找端口");
  for port in 1025..=65535 {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
    match TcpListener::bind(&addr).await {
      Ok(listener) => {
        #[cfg(debug_assertions)]
        println!("🔄 尝试绑定使用端口 {} ", port);
        return Ok((addr, listener));
      }
      Err(_) => {
        #[cfg(debug_assertions)]
        println!("🔄 端口 {} 不可用，尝试下一个", port);
        continue;
      }
    }
  }

  Err("无法找到可用端口".into())
}

// 图标加载
#[cfg(target_os = "windows")]
fn load_icon_from_file<P: AsRef<Path>>(
  path: P,
) -> Result<tauri::image::Image<'static>, Box<dyn std::error::Error>> {
  use image::ImageReader;
  use std::fs::File;
  use std::io::BufReader;

  let file = File::open(path.as_ref())?;
  let reader = BufReader::new(file);

  let icon_image = ImageReader::new(reader).with_guessed_format()?.decode()?;
  let rgba = icon_image.to_rgba8();
  let (width, height) = rgba.dimensions();
  let rgba_data = rgba.into_raw();
  let slice: &'static [u8] = Box::leak(rgba_data.into_boxed_slice());

  Ok(tauri::image::Image::new(slice, width, height))
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

  // 对 URL 路径进行百分号解码，处理中文等非 ASCII 字符
  let decoded_path = percent_encoding::percent_decode_str(url_path)
    .decode_utf8()
    .unwrap_or_else(|_| url_path.into());

  // 调试信息：输出请求的路径
  // #[cfg(debug_assertions)]
  // println!("请求路径: {}", decoded_path);

  // 处理根路径并清理URL路径
  let clean_path = if decoded_path == "/" {
    // 如果是根路径，则映射到index.html
    "index.html".to_string()
  } else {
    // 否则移除路径开头的斜杠
    decoded_path.trim_start_matches('/').to_string()
  };

  // 构造完整文件路径
  let file_path = base_path.join(&clean_path);

  // 调试信息：输出实际文件路径
  // #[cfg(debug_assertions)]
  // println!("实际文件路径: {:?}", file_path);

  // 检查路径是否在基础目录内，防止路径穿越攻击
  if !file_path.starts_with(&base_path) {
    // 如果路径不在基础目录内，返回禁止访问响应
    return forbidden_response(&file_path);
  }

  // 检查文件是否存在
  if !file_path.exists() {
    // 如果文件不存在，返回404响应
    return not_found_response(&file_path);
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
      // #[cfg(debug_assertions)]
      // println!("成功返回文件: {:?}, 类型: {}", file_path, content_type);

      // 构造成功响应，添加缓存控制头
      Ok(
        Response::builder()
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
          .unwrap(),
      )
    }
    // 读取文件失败
    Err(_) => not_found_response(&file_path),
  }
}

// 返回404未找到响应
fn not_found_response(_file_path: &std::path::Path) -> Result<Response<Full<Bytes>>, Infallible> {
  // 调试信息：输出返回404
  #[cfg(debug_assertions)]
  println!("❌ 404 - 文件不存在：{:?}", _file_path);
  // 构造404响应
  Ok(
    Response::builder()
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
      .unwrap(),
  )
}

// 返回403禁止访问响应
fn forbidden_response(_file_path: &std::path::Path) -> Result<Response<Full<Bytes>>, Infallible> {
  // 调试信息：输出返回403
  #[cfg(debug_assertions)]
  println!("❌ 403 - 文件禁止访问：{:?}", _file_path);
  // 构造403响应
  Ok(
    Response::builder()
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
      .unwrap(),
  )
}
