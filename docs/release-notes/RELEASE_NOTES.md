## @just-every/code v0.5.15
自动化钩子进一步收紧发布流程，CLI 获取最新二进制。

### 变更
- CLI：将 npm 元数据升级到 0.5.15，确保新安装拉取最新二进制。
- CI：在推送到 main 前强制运行 `./pre-release.sh`，保证发布检查通过。

### 安装
```
npm install -g @just-every/code@latest
code
```

对比：https://github.com/just-every/code/compare/v0.5.14...v0.5.15
