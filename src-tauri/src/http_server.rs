// HTTP 服务器模块
// 提供端口查找和静态文件服务功能

use std::{
  convert::Infallible, // 用于错误处理的类型
  path::PathBuf,       // 路径缓冲区类型
};

use bytes::Bytes; // 导入字节操作
use http_body_util::Full; // 导入 HTTP 消息体工具
use hyper::{Request, Response, StatusCode}; // 导入 Request、Response 和状态码

// 静态文件服务函数，处理 HTTP 请求
pub async fn serve_static(
  // HTTP 请求参数
  req: Request<hyper::body::Incoming>,
  // 静态文件根目录路径
  base_path: PathBuf,
) -> Result<Response<Full<Bytes>>, Infallible> {
  // 获取请求的 URI 路径
  let url_path = req.uri().path();

  // 对 URL 路径进行百分号解码，处理中文等非 ASCII 字符
  let decoded_path = percent_encoding::percent_decode_str(url_path)
    .decode_utf8()
    .unwrap_or_else(|_| url_path.into());

  // 调试信息：输出请求的路径
  // #[cfg(debug_assertions)]
  // println!("请求路径：{}", decoded_path);

  // 处理根路径并清理 URL 路径
  let clean_path = if decoded_path == "/" {
    // 如果是根路径，则映射到 index.html
    "index.html".to_string()
  } else {
    // 否则移除路径开头的斜杠
    decoded_path.trim_start_matches('/').to_string()
  };

  // 构造完整文件路径
  let file_path = base_path.join(&clean_path);

  // 调试信息：输出实际文件路径
  // #[cfg(debug_assertions)]
  // println!("实际文件路径：{:?}", file_path);

  // 检查路径是否在基础目录内，防止路径穿越攻击
  if !file_path.starts_with(&base_path) {
    // 如果路径不在基础目录内，返回禁止访问响应
    return forbidden_response(&file_path);
  }

  // 检查文件是否存在
  if !file_path.exists() {
    // 如果文件不存在，返回 404 响应
    return not_found_response(&file_path);
  }

  // 读取文件内容
  match tokio::fs::read(&file_path).await {
    // 成功读取文件
    Ok(contents) => {
      // 将内容包装为 Full<Bytes> 类型
      let body = Full::from(contents);
      // 根据文件扩展名确定内容类型
      let content_type = match file_path.extension().and_then(|ext| ext.to_str()) {
        // HTML 文件类型
        Some("html") | Some("htm") => "text/html",
        // CSS 样式文件类型
        Some("css") => "text/css",
        // JavaScript 文件类型
        Some("js") => "application/javascript",
        // JSON 文件类型
        Some("json") => "application/json",
        // PNG 图片类型
        Some("png") => "image/png",
        // JPEG 图片类型
        Some("jpg") | Some("jpeg") => "image/jpeg",
        // GIF 图片类型
        Some("gif") => "image/gif",
        // SVG 图片类型
        Some("svg") => "image/svg+xml",
        // ICO 图标类型
        Some("ico") => "image/x-icon",
        // WOFF 字体类型
        Some("woff") => "font/woff",
        // WOFF2 字体类型
        Some("woff2") => "font/woff2",
        // TTF 字体类型
        Some("ttf") => "font/ttf",
        // EOT 字体类型
        Some("eot") => "application/vnd.ms-fontobject",
        // 默认文本类型
        _ => "text/plain",
      };

      // 调试信息：输出成功返回的文件及类型
      // #[cfg(debug_assertions)]
      // println!("成功返回文件：{:?}, 类型：{}", file_path, content_type);

      // 构造成功响应，添加缓存控制头
      Ok(
        Response::builder()
          // 设置状态码为 200 OK
          .status(StatusCode::OK)
          // 设置内容类型头部
          .header("Content-Type", content_type)
          // 设置 CORS 头部允许所有来源
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

// 返回 404 未找到响应
pub fn not_found_response(
  _file_path: &std::path::Path,
) -> Result<Response<Full<Bytes>>, Infallible> {
  // 调试信息：输出返回 404
  #[cfg(debug_assertions)]
  println!("❌ 404 - 文件不存在：{:?}", _file_path);
  // 构造 404 响应
  Ok(
    Response::builder()
      // 设置状态码为 404
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

// 返回 403 禁止访问响应
pub fn forbidden_response(
  _file_path: &std::path::Path,
) -> Result<Response<Full<Bytes>>, Infallible> {
  // 调试信息：输出返回 403
  #[cfg(debug_assertions)]
  println!("❌ 403 - 文件禁止访问：{:?}", _file_path);
  // 构造 403 响应
  Ok(
    Response::builder()
      // 设置状态码为 403
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
