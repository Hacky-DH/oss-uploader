# OSS Uploader - Rust 版本

一个用 Rust 编写的 OSS/S3 对象存储上传下载工具，兼容 AWS S3 API。

## 功能特性

- **上传**: 支持单文件上传和分块上传（自动检测文件大小）
- **下载**: 从 OSS 下载文件到本地
- **删除**: 删除 OSS 上的文件
- **并发上传**: 大文件自动使用多线程分块上传
- **进度显示**: 上传时显示进度条

## 安装

### 从源码编译

```bash
# 克隆仓库
cd oss-uploader

# 编译当前平台
cargo build --release

# 或使用 Makefile
make build
```

### 交叉编译

```bash
# macOS Intel + Apple Silicon
make build-mac

# Linux x86_64 + ARM64
make build-linux

# 所有平台
make build-all

# 或使用脚本
chmod +x build.sh
./build.sh x86_64-apple-darwin
./build.sh x86_64-apple-darwin aarch64-unknown-linux-musl  # macOS + Linux 静态链接
```

## 配置

通过环境变量配置 OSS 连接信息（**所有配置项都是必需的**）：

```bash
export OSS_ACCESS_KEY="your-access-key"              # 必需: OSS Access Key
export OSS_SECRET_KEY="your-secret-key"              # 必需: OSS Secret Key
export OSS_BUCKET="your-bucket"                      # 必需: OSS Bucket 名称
export OSS_ENDPOINT="https://s3.com"  # 必需: OSS Endpoint URL
export OSS_REGION=""                       # 必需: OSS Region
```

## 使用方法

### 上传文件

```bash
# 基本用法（key 默认为 <prefix>/filename）
oss-uploader upload /path/to/file.txt

# 指定 key
oss-uploader upload /path/to/file.txt -k myfolder/file.txt
```

### 下载文件

```bash
# 下载到当前目录
oss-uploader download myfolder/file.txt

# 指定输出路径
oss-uploader download myfolder/file.txt -o /path/to/save/
```

### 删除文件

```bash
oss-uploader delete myfolder/file.txt
```

## 项目结构

```
oss-uploader/
├── Cargo.toml           # 项目配置
├── Makefile            # 构建脚本
├── build.sh            # 交叉编译脚本
├── README.md           # 本文件
├── src/
│   ├── main.rs         # 主程序入口
│   └── lib.rs          # 核心库
├── tests/
│   └── integration_tests.rs  # 集成测试
└── .cargo/
    └── config.toml     # Cargo 配置（交叉编译）
```

## 测试

```bash
# 运行单元测试
cargo test

# 运行集成测试（需要真实 OSS 凭证）
export OSS_ACCESS_KEY=...
export OSS_SECRET_KEY=...
cargo test --test integration_tests -- --ignored
```

## 编译目标

支持以下平台的交叉编译（Linux 使用 musl 静态链接）：

| 平台 | Target | 命令 | 说明 |
|------|--------|------|------|
| macOS Intel | x86_64-apple-darwin | `make build-mac-intel` | 动态链接 |
| macOS ARM | aarch64-apple-darwin | `make build-mac-arm` | 动态链接 |
| Linux x64 | x86_64-unknown-linux-musl | `make build-linux-x64` | **静态链接**，不依赖 glibc |
| Linux ARM | aarch64-unknown-linux-musl | `make build-linux-arm` | **静态链接**，不依赖 glibc |

### musl 静态链接的优势

Linux 版本使用 musl 静态链接，生成完全独立的二进制文件：

- ✅ 不依赖系统的 glibc，可在任何 Linux 发行版上运行
- ✅ Alpine Linux 完美支持
- ✅ 容器化部署无需额外依赖
- ✅ 二进制文件更便携

## 许可证

MIT License
