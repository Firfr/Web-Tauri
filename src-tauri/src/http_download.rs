// 文件路径处理（Path 为借用，PathBuf 为拥有）
use std::path::{Path, PathBuf};
// 导入下载事件类型
use tauri::webview::DownloadEvent;
// 导入对话框插件
use tauri_plugin_dialog::DialogExt;

// 创建下载事件处理器
pub fn create_download_handler(
  handle: tauri::AppHandle,
) -> impl Fn(tauri::webview::Webview, DownloadEvent<'_>) -> bool + Send + Sync + 'static {
  move |_webview, event| match event {
    DownloadEvent::Requested { url, destination } => {
      handle_download_requested(&handle, &url, destination)
    }
    DownloadEvent::Finished { url, path, success } => {
      handle_download_finished(&url, path.as_deref(), success);
      true
    }
    _ => true,
  }
}

// 处理下载请求事件
pub fn handle_download_requested(
  handle: &tauri::AppHandle,
  url: &tauri::Url,
  destination: &mut PathBuf,
) -> bool {
  #[cfg(debug_assertions)]
  println!("📥 下载请求：{}", url);

  // 从 URL 提取默认文件名
  let extracted_name = extract_filename_from_url(url.as_str());

  let default_filename = "请设置文件名和后缀";

  // 根据提取结果决定默认文件名
  let default_filename = match extracted_name {
    Some(name) => {
      // 检查是否是 UUID 格式（blob URL 常见）
      if is_uuid_format(&name) {
        default_filename.to_string()
      } else {
        name
      }
    }
    None => default_filename.to_string(),
  };

  #[cfg(debug_assertions)]
  println!("📄 默认文件名：{}", default_filename);

  // 显示保存对话框
  let save_path = handle
    .dialog()
    .file()
    .set_file_name(&default_filename)
    .set_title("保存文件")
    .blocking_save_file();

  // 如果用户选择了保存位置
  if let Some(path) = save_path {
    #[cfg(debug_assertions)]
    println!("✅ 用户选择保存路径：{:?}", path);

    // 设置下载目标路径
    if let Some(path_ref) = path.as_path() {
      *destination = path_ref.to_owned();
    }

    return true;
  }

  #[cfg(debug_assertions)]
  println!("❌ 用户取消了下载");
  false
}

// 处理下载完成事件
pub fn handle_download_finished(_url: &tauri::Url, _path: Option<&Path>, success: bool) {
  if success {
    #[cfg(debug_assertions)]
    println!("✅ 下载完成：{} -> {:?}", _url, _path);
  } else {
    #[cfg(debug_assertions)]
    println!("❌ 下载失败：{} -> {:?}", _url, _path);
  }
}

// 从 URL 提取文件名的辅助函数
pub fn extract_filename_from_url(url: &str) -> Option<String> {
  // 处理 blob: URL
  if url.starts_with("blob:") {
    // blob URL 通常格式为 blob:http://origin/uuid
    // 这种情况下无法从 URL 获取文件名，返回 None
    return None;
  }

  // 尝试解析 URL
  if let Ok(parsed) = url::Url::parse(url) {
    // 获取路径部分
    let path = parsed.path();

    // 从路径中提取文件名
    if let Some(filename) = Path::new(path).file_name() {
      if let Some(name) = filename.to_str() {
        if !name.is_empty() {
          return Some(name.to_string());
        }
      }
    }
  }

  // 如果 URL 解析失败或没有文件名，尝试从字符串直接提取
  if let Some(pos) = url.rfind('/') {
    let filename = &url[pos + 1..];
    if !filename.is_empty() && !filename.contains('?') && !filename.contains('#') {
      return Some(filename.to_string());
    }
  }

  None
}

// 检查字符串是否是 UUID 格式
pub fn is_uuid_format(s: &str) -> bool {
  // UUID 格式：8-4-4-4-12 的十六进制字符
  // 例如：6d72c65d-8c88-44d6-822a-d16dc896c740
  if s.len() != 36 {
    return false;
  }

  let parts: Vec<&str> = s.split('-').collect();
  if parts.len() != 5 {
    return false;
  }

  // 检查每部分的长度
  let expected_lengths = [8, 4, 4, 4, 12];
  for (i, part) in parts.iter().enumerate() {
    if part.len() != expected_lengths[i] {
      return false;
    }
    // 检查是否都是十六进制字符
    if !part.chars().all(|c| c.is_ascii_hexdigit()) {
      return false;
    }
  }

  true
}
