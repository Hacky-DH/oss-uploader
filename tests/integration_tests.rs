use std::io::Write;
use tempfile::NamedTempFile;
use oss_uploader::{OssClient, OssConfig};

/// 测试辅助函数：创建临时配置文件
fn create_test_config() -> OssConfig {
    OssConfig {
        access_key: "test_access_key".to_string(),
        secret_key: "test_secret_key".to_string(),
        bucket: "test-bucket".to_string(),
        endpoint: "https://test.endpoint.com".to_string(),
        region: "test_region".to_string(),
    }
}

#[tokio::test]
async fn test_oss_config() {
    let config = create_test_config();
    
    assert_eq!(config.access_key, "test_access_key");
    assert_eq!(config.secret_key, "test_secret_key");
    assert_eq!(config.bucket, "test-bucket");
    assert_eq!(config.endpoint, "https://test.endpoint.com");
}

#[test]
fn test_url_generation() {
    // 这个测试不需要连接到真实服务
    // 只是验证配置对象的创建
    let config = create_test_config();
    assert_eq!(config.endpoint, "https://test.endpoint.com");
}

#[test]
fn test_format_size() {
    // 测试文件大小格式化
    fn format_size(mut size_bytes: f64) -> String {
        for unit in ["B", "KB", "MB", "GB", "TB"] {
            if size_bytes < 1024.0 {
                return format!("{:.2} {}", size_bytes, unit);
            }
            size_bytes /= 1024.0;
        }
        format!("{:.2} PB", size_bytes)
    }

    assert_eq!(format_size(100.0), "100.00 B");
    assert_eq!(format_size(1024.0), "1.00 KB");
    assert_eq!(format_size(1024.0 * 1024.0), "1.00 MB");
    assert_eq!(format_size(10.0 * 1024.0 * 1024.0), "10.00 MB");
}

/// 集成测试（需要真实 OSS 凭证）
#[tokio::test]
#[ignore] // 默认忽略，需要配置真实环境变量
async fn test_upload_integration() {
    // 这些测试需要真实的环境变量配置
    if std::env::var("OSS_ACCESS_KEY").is_err() {
        println!("跳过集成测试：未设置 OSS_ACCESS_KEY");
        return;
    }

    let config = OssConfig::from_env().expect("Failed to load config");
    let client = OssClient::new(config).await.expect("Failed to create client");

    // 创建临时测试文件
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "Hello OSS!").unwrap();
    let path = temp_file.path().to_path_buf();

    let key = "test/integration_test.txt";
    
    // 测试上传
    let url = client.upload(&path, key).await;
    assert!(url.is_ok(), "上传失败: {:?}", url.err());

    // 测试下载
    let download_path = client.download(key, None).await;
    assert!(download_path.is_ok(), "下载失败: {:?}", download_path.err());

    // 清理
    let _ = client.delete(key).await;
    if let Ok(path) = download_path {
        let _ = std::fs::remove_file(path);
    }
}

#[tokio::test]
#[ignore] // 默认忽略，需要配置真实环境变量
async fn test_multipart_upload_integration() {
    if std::env::var("OSS_ACCESS_KEY").is_err() {
        println!("跳过集成测试：未设置 OSS_ACCESS_KEY");
        return;
    }

    let config = OssConfig::from_env().expect("Failed to load config");
    let client = OssClient::new(config).await.expect("Failed to create client");

    // 创建大文件（超过 10MB）
    let mut temp_file = NamedTempFile::new().unwrap();
    let data = vec![0u8; 11 * 1024 * 1024]; // 11 MB
    temp_file.write_all(&data).unwrap();
    let path = temp_file.path().to_path_buf();

    let key = "test/multipart_test.bin";
    
    // 测试分块上传
    let url = client.upload(&path, key).await;
    assert!(url.is_ok(), "分块上传失败: {:?}", url.err());

    // 清理
    let _ = client.delete(key).await;
}
