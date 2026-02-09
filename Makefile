# Makefile for oss-uploader
# macOS 用户需要先安装 musl 交叉编译器:
#   brew install FiloSottile/musl-cross/musl-cross

.PHONY: build build-mac build-linux build-all test clean install check-musl

PROJECT_NAME = oss-uploader
VERSION = $(shell grep '^version' Cargo.toml | head -n1 | cut -d'"' -f2)

# 检查 musl-gcc 是否安装
define check_musl_gcc
	@if ! command -v $(1) &> /dev/null; then \
		echo "错误: 未找到 $(1)"; \
		echo ""; \
		echo "请在 macOS 上安装 musl 交叉编译器:"; \
		echo "  brew install FiloSottile/musl-cross/musl-cross"; \
		echo ""; \
		echo "或者使用 cross 工具（需要 Docker）:"; \
		echo "  cargo install cross --git https://github.com/cross-rs/cross"; \
		echo "  cross build --release --target $(2)"; \
		exit 1; \
	fi
endef

# 默认构建当前平台
build:
	cargo build --release

# 构建 macOS Intel
build-mac-intel:
	rustup target add x86_64-apple-darwin
	cargo build --release --target x86_64-apple-darwin

# 构建 macOS Apple Silicon
build-mac-arm:
	rustup target add aarch64-apple-darwin
	cargo build --release --target aarch64-apple-darwin

# 构建所有 macOS 版本
build-mac: build-mac-intel build-mac-arm

# 构建 Linux x86_64 (musl 静态链接)
build-linux-x64:
	$(call check_musl_gcc,x86_64-linux-musl-gcc,x86_64-unknown-linux-musl)
	rustup target add x86_64-unknown-linux-musl
	CC_x86_64_unknown_linux_musl=x86_64-linux-musl-gcc \
	CXX_x86_64_unknown_linux_musl=x86_64-linux-musl-g++ \
	RUSTFLAGS="-C target-feature=+crt-static" \
	cargo build --release --target x86_64-unknown-linux-musl

# 构建 Linux ARM64 (musl 静态链接)
build-linux-arm:
	$(call check_musl_gcc,aarch64-linux-musl-gcc,aarch64-unknown-linux-musl)
	rustup target add aarch64-unknown-linux-musl
	CC_aarch64_unknown_linux_musl=aarch64-linux-musl-gcc \
	CXX_aarch64_unknown_linux_musl=aarch64-linux-musl-g++ \
	RUSTFLAGS="-C target-feature=+crt-static" \
	cargo build --release --target aarch64-unknown-linux-musl

# 构建所有 Linux 版本
build-linux: build-linux-x64 build-linux-arm

# 构建所有平台
build-all: build-mac build-linux

# 运行测试
test:
	cargo test

# 运行集成测试（需要真实 OSS 凭证）
test-integration:
	cargo test --test integration_tests -- --ignored

# 清理构建产物
clean:
	cargo clean
	rm -rf releases/

# 本地安装
install:
	cargo install --path .

# 创建发布包
release-mac-intel: build-mac-intel
	mkdir -p releases/$(PROJECT_NAME)-$(VERSION)-x86_64-apple-darwin
	cp target/x86_64-apple-darwin/release/$(PROJECT_NAME) releases/$(PROJECT_NAME)-$(VERSION)-x86_64-apple-darwin/
	cp README.md releases/$(PROJECT_NAME)-$(VERSION)-x86_64-apple-darwin/ 2>/dev/null || true
	tar czf releases/$(PROJECT_NAME)-$(VERSION)-x86_64-apple-darwin.tar.gz -C releases $(PROJECT_NAME)-$(VERSION)-x86_64-apple-darwin

release-mac-arm: build-mac-arm
	mkdir -p releases/$(PROJECT_NAME)-$(VERSION)-aarch64-apple-darwin
	cp target/aarch64-apple-darwin/release/$(PROJECT_NAME) releases/$(PROJECT_NAME)-$(VERSION)-aarch64-apple-darwin/
	cp README.md releases/$(PROJECT_NAME)-$(VERSION)-aarch64-apple-darwin/ 2>/dev/null || true
	tar czf releases/$(PROJECT_NAME)-$(VERSION)-aarch64-apple-darwin.tar.gz -C releases $(PROJECT_NAME)-$(VERSION)-aarch64-apple-darwin

release-linux-x64: build-linux-x64
	mkdir -p releases/$(PROJECT_NAME)-$(VERSION)-x86_64-unknown-linux-musl
	cp target/x86_64-unknown-linux-musl/release/$(PROJECT_NAME) releases/$(PROJECT_NAME)-$(VERSION)-x86_64-unknown-linux-musl/
	cp README.md releases/$(PROJECT_NAME)-$(VERSION)-x86_64-unknown-linux-musl/ 2>/dev/null || true
	tar czf releases/$(PROJECT_NAME)-$(VERSION)-x86_64-unknown-linux-musl.tar.gz -C releases $(PROJECT_NAME)-$(VERSION)-x86_64-unknown-linux-musl

release-linux-arm: build-linux-arm
	mkdir -p releases/$(PROJECT_NAME)-$(VERSION)-aarch64-unknown-linux-musl
	cp target/aarch64-unknown-linux-musl/release/$(PROJECT_NAME) releases/$(PROJECT_NAME)-$(VERSION)-aarch64-unknown-linux-musl/
	cp README.md releases/$(PROJECT_NAME)-$(VERSION)-aarch64-unknown-linux-musl/ 2>/dev/null || true
	tar czf releases/$(PROJECT_NAME)-$(VERSION)-aarch64-unknown-linux-musl.tar.gz -C releases $(PROJECT_NAME)-$(VERSION)-aarch64-unknown-linux-musl

release-all: release-mac-intel release-mac-arm release-linux-x64 release-linux-arm

# 帮助信息
help:
	@echo "Available targets:"
	@echo "  build              - Build for current platform"
	@echo "  build-mac-intel    - Build for macOS Intel (x86_64)"
	@echo "  build-mac-arm      - Build for macOS ARM (Apple Silicon)"
	@echo "  build-mac          - Build for all macOS platforms"
	@echo "  build-linux-x64    - Build for Linux x86_64 (musl static)"
	@echo "  build-linux-arm    - Build for Linux ARM64 (musl static)"
	@echo "  build-linux        - Build for all Linux platforms"
	@echo "  build-all          - Build for all platforms"
	@echo "  test               - Run unit tests"
	@echo "  test-integration   - Run integration tests (needs OSS credentials)"
	@echo "  clean              - Clean build artifacts"
	@echo "  install            - Install locally with cargo"
	@echo "  release-all        - Create release packages for all platforms"
	@echo ""
	@echo "Prerequisites for Linux cross-compilation on macOS:"
	@echo "  brew install FiloSottile/musl-cross/musl-cross"
