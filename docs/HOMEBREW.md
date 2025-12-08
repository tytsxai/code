Homebrew（macOS）

仓库包含一个脚本，可基于最新 GitHub Release 产物生成 Homebrew formula。发布到 Homebrew 需要一个 tap 仓库（如 `just-every/homebrew-tap`）。tap 就绪后按以下步骤生成并发布：

1) 为最新版本生成 formula：

```
scripts/generate-homebrew-formula.sh
```

2) 将生成的 `Code.rb` 复制到 tap 仓库下的 `Formula/Code.rb`，必要时更新 `url`/`sha256`。

3) 用户即可通过：

```
brew tap just-every/tap
brew install code
```

注意事项

- formula 期望的发布资源名称：
  - `code-aarch64-apple-darwin.tar.gz`
  - `code-x86_64-apple-darwin.tar.gz`
- CLI 会安装 `code` 与 `coder` 两个 shim 以保持兼容。
