#!/bin/bash

# 交叉编译脚本 - 支持 macOS 和 Linux (musl 静态链接)
# 使用方法: ./build.sh [target]
# 如果不指定 target，则编译当前平台
# macOS 用户需要先安装: brew install FiloSottile/musl-cross/musl-cross

set -e

PROJECT_NAME="oss-uploader"
VERSION=$(grep '^version' Cargo.toml | head -n1 | cut -d'"' -f2)

echo "Building $PROJECT_NAME v$VERSION"

# 检查 musl-gcc 是否安装
check_musl_gcc() {
    local arch=$1
    local gcc_name="${arch}-linux-musl-gcc"
    
    if ! command -v "$gcc_name" &> /dev/null; then
        echo "错误: 未找到 $gcc_name"
        echo ""
        echo "请在 macOS 上安装 musl 交叉编译器:"
        echo "  brew install FiloSottile/musl-cross/musl-cross"
        echo ""
        echo "或者使用 cross 工具（需要 Docker）:"
        echo "  cargo install cross --git https://github.com/cross-rs/cross"
        echo "  cross build --release --target ${arch}-unknown-linux-musl"
        exit 1
    fi
}

# 安装 musl 目标的函数
install_musl_target() {
    echo "Installing musl targets..."
    rustup target add x86_64-unknown-linux-musl 2>/dev/null || true
    rustup target add aarch64-unknown-linux-musl 2>/dev/null || true
}

# 编译函数
build_target() {
    local target=$1
    local target_dir="target/$target/release"
    
    echo ""
    echo "Building for target: $target"
    
    # 添加目标
    rustup target add "$target" 2>/dev/null || true
    
    # 检查是否为 musl 目标
    if [[ "$target" == *"musl"* ]]; then
        echo "Using musl static linking..."
        
        # 确定架构并检查对应的 gcc
        if [[ "$target" == "x86_64-unknown-linux-musl" ]]; then
            check_musl_gcc "x86_64"
            export CC_x86_64_unknown_linux_musl=x86_64-linux-musl-gcc
            export CXX_x86_64_unknown_linux_musl=x86_64-linux-musl-g++
        elif [[ "$target" == "aarch64-unknown-linux-musl" ]]; then
            check_musl_gcc "aarch64"
            export CC_aarch64_unknown_linux_musl=aarch64-linux-musl-gcc
            export CXX_aarch64_unknown_linux_musl=aarch64-linux-musl-g++
        fi
        
        # 设置静态链接标志
        export RUSTFLAGS="-C target-feature=+crt-static"
        
        cargo build --release --target "$target"
    else
        cargo build --release --target "$target"
    fi
    
    # 创建发布目录
    local release_dir="releases/${PROJECT_NAME}-${VERSION}-${target}"
    mkdir -p "$release_dir"
    
    # 复制二进制文件
    if [[ "$target" == *"windows"* ]]; then
        cp "$target_dir/$PROJECT_NAME.exe" "$release_dir/" 2>/dev/null || \
        cp "$target_dir/${PROJECT_NAME//-/_}.exe" "$release_dir/"
    else
        cp "$target_dir/$PROJECT_NAME" "$release_dir/" 2>/dev/null || \
        cp "$target_dir/${PROJECT_NAME//-/_}" "$release_dir/"
    fi

    # 打包
    if command -v tar &> /dev/null; then
        tar czf "releases/${PROJECT_NAME}-${VERSION}-${target}.tar.gz" -C releases "${PROJECT_NAME}-${VERSION}-${target}"
        echo "Created: releases/${PROJECT_NAME}-${VERSION}-${target}.tar.gz"
    fi
    
    if command -v zip &> /dev/null && [[ "$target" == *"windows"* ]]; then
        cd releases && zip -r "${PROJECT_NAME}-${VERSION}-${target}.zip" "${PROJECT_NAME}-${VERSION}-${target}" && cd ..
        echo "Created: releases/${PROJECT_NAME}-${VERSION}-${target}.zip"
    fi
}

# 主逻辑
main() {
    # 创建发布目录
    mkdir -p releases
    
    # 检查是否指定了目标
    if [ $# -eq 0 ]; then
        # 编译当前平台
        echo "Building for current platform..."
        cargo build --release
        echo "Binary: target/release/$PROJECT_NAME"
    else
        # 安装 musl 目标（如果需要）
        for target in "$@"; do
            if [[ "$target" == *"musl"* ]]; then
                install_musl_target
                break
            fi
        done
        
        # 编译指定目标
        for target in "$@"; do
            build_target "$target"
        done
    fi
    
    echo ""
    echo "Build complete!"
    echo "Releases are in: releases/"
}

# 支持的编译目标
print_supported_targets() {
    echo "Supported targets:"
    echo "  x86_64-apple-darwin        (macOS Intel)"
    echo "  aarch64-apple-darwin       (macOS Apple Silicon)"
    echo "  x86_64-unknown-linux-musl  (Linux x86_64, musl static)"
    echo "  aarch64-unknown-linux-musl (Linux ARM64, musl static)"
    echo ""
    echo "Prerequisites for Linux cross-compilation on macOS:"
    echo "  brew install FiloSottile/musl-cross/musl-cross"
    echo ""
    echo "Usage examples:"
    echo "  ./build.sh                                           # Current platform only"
    echo "  ./build.sh x86_64-apple-darwin                       # macOS Intel"
    echo "  ./build.sh x86_64-unknown-linux-musl                 # Linux x86_64 static"
    echo "  ./build.sh x86_64-apple-darwin aarch64-apple-darwin  # Both macOS"
    echo "  ./build.sh x86_64-apple-darwin x86_64-unknown-linux-musl  # macOS & Linux"
}

# 处理参数
case "${1:-}" in
    --help|-h)
        print_supported_targets
        exit 0
        ;;
    *)
        main "$@"
        ;;
esac
