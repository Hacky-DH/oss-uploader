use clap::{Parser, Subcommand};
use std::path::PathBuf;
use anyhow::Result;

use oss_uploader::{OssClient, OssConfig};

#[derive(Parser)]
#[command(name = "oss-uploader")]
#[command(about = "OSS 上传下载工具 (兼容 S3 API)")]
#[command(version)]
struct Cli {
    /// 子命令
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 上传文件到 OSS
    Upload {
        /// 本地文件路径
        file_path: PathBuf,

        /// 远程 key（可选，默认为 <key_prefix>/<filename>）
        #[arg(short = 'k', long)]
        key: Option<String>,

        /// key 前缀（可选，默认为空，即直接放在根目录）
        #[arg(short = 'p', long)]
        key_prefix: Option<String>,
    },

    /// 从 OSS 下载文件
    Download {
        /// 远程 key
        key: String,

        /// 本地输出路径（可选，默认为 key 的文件名）
        #[arg(short = 'o', long)]
        output: Option<PathBuf>,
    },

    /// 删除 OSS 上的文件
    Delete {
        /// 远程 key
        key: String,
    },

    /// 生成预签名下载 URL（临时访问链接）
    Url {
        /// 远程 key
        key: String,

        /// URL 有效期（秒，默认 3600 = 1小时）
        #[arg(short = 'e', long, default_value = "3600")]
        expires: u64,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // 从环境变量读取配置
    let config = OssConfig::from_env()
        .map_err(|e| anyhow::anyhow!("配置错误: {}\n请确保设置了必需的环境变量", e))?;

    // 创建客户端
    let client = OssClient::new(config).await?;

    match cli.command {
        Commands::Upload { file_path, key, key_prefix } => {
            let key = key.unwrap_or_else(|| {
                let filename = file_path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy();
                match key_prefix {
                    Some(prefix) => format!("{}/{}", prefix.trim_end_matches('/'), filename),
                    None => filename.to_string(),
                }
            });

            println!("开始上传 {} ...", file_path.display());
            let url = client.upload(&file_path, &key).await?;
            println!("成功上传 {}\n下载 url:\n{}", file_path.display(), url);
        }
        
        Commands::Download { key, output } => {
            client.download(&key, output.as_deref()).await?;
        }
        
        Commands::Delete { key } => {
            client.delete(&key).await?;
        }

        Commands::Url { key, expires } => {
            let url = client.generate_presigned_url(&key, expires).await?;
            println!("{}", url);
        }
    }

    Ok(())
}
