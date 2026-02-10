use aws_config::BehaviorVersion;
use aws_sdk_s3::Client;
use aws_sdk_s3::types::CompletedPart;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use anyhow::{Result, Context};
use tokio::sync::{Mutex, Semaphore};

/// 分块大小 10MB
const BATCH_SIZE: usize = 10 * 1024 * 1024;
/// 最大并发数
const MAX_WORKERS: usize = 10;

/// OSS 配置
#[derive(Debug, Clone)]
pub struct OssConfig {
    pub access_key: String,
    pub secret_key: String,
    pub bucket: String,
    pub endpoint: String,
    pub region: String,
}

impl OssConfig {
    /// 从环境变量创建配置
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            access_key: std::env::var("OSS_ACCESS_KEY")
                .context("OSS_ACCESS_KEY not set")?,
            secret_key: std::env::var("OSS_SECRET_KEY")
                .context("OSS_SECRET_KEY not set")?,
            bucket: std::env::var("OSS_BUCKET")
                .context("OSS_BUCKET not set")?,
            endpoint: std::env::var("OSS_ENDPOINT")
                .context("OSS_ENDPOINT not set")?,
            region: std::env::var("OSS_REGION")
                .context("OSS_REGION not set")?,
        })
    }
}

/// OSS 客户端
pub struct OssClient {
    client: Client,
    config: OssConfig,
}

impl OssClient {
    /// 创建新的 OSS 客户端
    pub async fn new(config: OssConfig) -> Result<Self> {
        // 使用静态凭据创建配置
        let sdk_config = aws_config::defaults(BehaviorVersion::latest())
            .endpoint_url(&config.endpoint)
            .region(aws_sdk_s3::config::Region::new(config.region.clone()))
            .credentials_provider(
                aws_sdk_s3::config::Credentials::new(
                    &config.access_key,
                    &config.secret_key,
                    None,
                    None,
                    "env",
                )
            )
            .load()
            .await;

        let client = Client::new(&sdk_config);

        Ok(Self { client, config })
    }

    /// 上传文件
    pub async fn upload(&self, path: &Path, key: &str) -> Result<String> {
        let abs_path = path.canonicalize()
            .with_context(|| format!("无法找到文件: {}", path.display()))?;
        
        let metadata = tokio::fs::metadata(&abs_path).await?;
        let file_size = metadata.len();

        if file_size <= BATCH_SIZE as u64 {
            self.upload_single(&abs_path, key).await
        } else {
            self.upload_multipart(&abs_path, key).await
        }
    }

    /// 单文件上传
    async fn upload_single(&self, path: &Path, key: &str) -> Result<String> {
        let mut file = File::open(path).await?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await?;

        let body = aws_sdk_s3::primitives::ByteStream::from(buffer);

        self.client
            .put_object()
            .bucket(&self.config.bucket)
            .key(key)
            .body(body)
            .send()
            .await?;

        Ok(self.generate_url(key))
    }

    /// 分块上传
    async fn upload_multipart(&self, path: &Path, key: &str) -> Result<String> {
        let metadata = tokio::fs::metadata(path).await?;
        let file_size = metadata.len();
        let total_parts = ((file_size + BATCH_SIZE as u64 - 1) / BATCH_SIZE as u64) as usize;

        println!("分块上传 {} 到 {}", path.display(), key);

        // 创建分块上传
        let create_resp = self.client
            .create_multipart_upload()
            .bucket(&self.config.bucket)
            .key(key)
            .storage_class(aws_sdk_s3::types::StorageClass::Standard)
            .send()
            .await?;

        let upload_id = create_resp.upload_id()
            .context("无法获取 upload id")?
            .to_string();

        // 进度条
        let pb = ProgressBar::new(file_size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta}) {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );
        pb.set_message(format!("上传 {}", path.file_name()
            .unwrap_or_default()
            .to_string_lossy()));

        let pb = Arc::new(pb);
        let semaphore = Arc::new(Semaphore::new(MAX_WORKERS));

        // 读取文件所有数据
        let mut file = File::open(path).await?;
        let mut parts_data = Vec::with_capacity(total_parts);
        
        for part_num in 1..=total_parts {
            let mut buffer = vec![0u8; BATCH_SIZE];
            let bytes_read = file.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }
            buffer.truncate(bytes_read);
            parts_data.push((part_num, buffer));
        }

        // 并发上传分块
        let mut tasks = Vec::with_capacity(parts_data.len());
        let parts_data = Arc::new(Mutex::new(parts_data));

        for _ in 0..parts_data.lock().await.len() {
            let client = self.client.clone();
            let bucket = self.config.bucket.clone();
            let key = key.to_string();
            let upload_id = upload_id.clone();
            let parts_data = parts_data.clone();
            let pb = pb.clone();
            let semaphore = semaphore.clone();

            let task = tokio::spawn(async move {
                let _permit = semaphore.acquire().await?;
                
                let (part_number, data) = {
                    let mut parts = parts_data.lock().await;
                    if parts.is_empty() {
                        return Ok::<Option<CompletedPart>, anyhow::Error>(None);
                    }
                    parts.remove(0)
                };

                let body = aws_sdk_s3::primitives::ByteStream::from(data.clone());

                let resp = client
                    .upload_part()
                    .bucket(&bucket)
                    .key(&key)
                    .part_number(part_number as i32)
                    .upload_id(&upload_id)
                    .body(body)
                    .send()
                    .await?;

                pb.inc(data.len() as u64);

                Ok(Some(
                    CompletedPart::builder()
                        .part_number(part_number as i32)
                        .e_tag(resp.e_tag().unwrap_or_default())
                        .build()
                ))
            });

            tasks.push(task);
        }

        // 收集结果
        let mut completed_parts = Vec::new();
        for task in tasks {
            if let Some(part) = task.await?? {
                completed_parts.push(part);
            }
        }

        pb.finish_with_message("上传完成");

        // 按 PartNumber 排序
        completed_parts.sort_by_key(|p| p.part_number());

        // 完成上传
        let completed_parts_obj = aws_sdk_s3::types::CompletedMultipartUpload::builder()
            .set_parts(Some(completed_parts))
            .build();

        self.client
            .complete_multipart_upload()
            .bucket(&self.config.bucket)
            .key(key)
            .upload_id(&upload_id)
            .multipart_upload(completed_parts_obj)
            .send()
            .await?;

        Ok(self.generate_url(key))
    }

    /// 下载文件
    pub async fn download(&self, key: &str, output_path: Option<&Path>) -> Result<PathBuf> {
        let output_path = output_path
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| {
                PathBuf::from(Path::new(key).file_name()
                    .unwrap_or_default())
            });

        let resp = self.client
            .get_object()
            .bucket(&self.config.bucket)
            .key(key)
            .send()
            .await?;

        let mut file = File::create(&output_path).await?;
        let mut stream = resp.body;

        while let Some(chunk) = stream.try_next().await? {
            file.write_all(&chunk).await?;
        }

        file.flush().await?;
        println!("成功下载 {} 到 {}", key, output_path.display());

        Ok(output_path)
    }

    /// 删除文件
    pub async fn delete(&self, key: &str) -> Result<()> {
        self.client
            .delete_object()
            .bucket(&self.config.bucket)
            .key(key)
            .send()
            .await?;

        println!("成功删除 {}", key);
        Ok(())
    }

    /// 生成下载 URL（使用 SDK 的 presigned 方法生成带签名的临时 URL）
    /// 适用于私有 bucket，生成有时效性的访问链接
    pub async fn generate_presigned_url(&self, key: &str, expires_in_secs: u64) -> Result<String> {
        use aws_sdk_s3::presigning::PresigningConfig;
        use std::time::Duration;

        let presigning_config = PresigningConfig::builder()
            .expires_in(Duration::from_secs(expires_in_secs))
            .build()?;

        let presigned_request = self
            .client
            .get_object()
            .bucket(&self.config.bucket)
            .key(key)
            .presigned(presigning_config)
            .await?;

        Ok(presigned_request.uri().to_string())
    }

    /// 生成简单的公开访问 URL（不带签名）
    /// 适用于公开可读的 bucket
    fn generate_url(&self, key: &str) -> String {
        let encoded_key = urlencoding::encode(key).replace("%2F", "/");
        let endpoint = self.config.endpoint.trim_end_matches('/');
        
        // 将 bucket 作为子域名插入到 endpoint 中
        if let Some(pos) = endpoint.find("://") {
            let protocol = &endpoint[..pos + 3];
            let domain = &endpoint[pos + 3..];
            format!("{}{}.{}/{}", protocol, self.config.bucket, domain, encoded_key)
        } else {
            // 如果没有协议前缀，直接使用 bucket 作为前缀
            format!("{}.{}/{}", self.config.bucket, endpoint, encoded_key)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oss_config_from_env() {
        // 设置所有必需的环境变量
        std::env::set_var("OSS_ACCESS_KEY", "test_key");
        std::env::set_var("OSS_SECRET_KEY", "test_secret");
        std::env::set_var("OSS_BUCKET", "test_bucket");
        std::env::set_var("OSS_ENDPOINT", "https://test.endpoint.com");
        std::env::set_var("OSS_REGION", "test_region");
        
        let config = OssConfig::from_env();
        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.access_key, "test_key");
        assert_eq!(config.secret_key, "test_secret");
        assert_eq!(config.bucket, "test_bucket");
        assert_eq!(config.endpoint, "https://test.endpoint.com");
        assert_eq!(config.region, "test_region");
    }
}
