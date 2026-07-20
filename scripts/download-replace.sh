#!/usr/bin/env bash
# 从 GitHub Actions 下载 main 分支最新成功的 Linux x86_64 构建并替换当前 Grok。
#
# 用法：
#   ./scripts/download-replace.sh
#   ./scripts/download-replace.sh --repo owner/repo
#   ./scripts/download-replace.sh --run-id 29720328041
#
# 环境变量：
#   GROK_GITHUB_REPO       GitHub 仓库，默认 liansishen/grok-build
#   GROK_INSTALL_DIR       安装目录，默认 ~/.grok/bin
#   GROK_WORKFLOW          工作流文件名，默认 build.yml
#   GROK_LINUX_ARTIFACT    Artifact 名，默认 grok-build-linux-x86_64

set -Eeuo pipefail

repo="${GROK_GITHUB_REPO:-liansishen/grok-build}"
install_dir="${GROK_INSTALL_DIR:-$HOME/.grok/bin}"
workflow="${GROK_WORKFLOW:-build.yml}"
artifact_name="${GROK_LINUX_ARTIFACT:-grok-build-linux-x86_64}"
run_id=""

usage() {
    cat <<'EOF'
下载 GitHub Actions 最新成功的 Linux x86_64 Grok 构建并替换当前版本。

用法：
  ./scripts/download-replace.sh [选项]

选项：
  --repo OWNER/REPO   GitHub 仓库（默认：liansishen/grok-build）
  --run-id ID         下载指定的成功工作流；默认查找 main 最新成功构建
  -h, --help          显示帮助
EOF
}

while (($#)); do
    case "$1" in
        --repo)
            [[ $# -ge 2 ]] || { echo "错误：--repo 缺少参数" >&2; exit 2; }
            repo="$2"
            shift 2
            ;;
        --run-id)
            [[ $# -ge 2 ]] || { echo "错误：--run-id 缺少参数" >&2; exit 2; }
            run_id="$2"
            if [[ ! "$run_id" =~ ^[0-9]+$ ]]; then
                echo "错误：--run-id 必须是完整的十进制数字，不能使用科学计数法：$run_id" >&2
                exit 2
            fi
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "错误：未知参数 $1" >&2
            usage >&2
            exit 2
            ;;
    esac
done

command -v gh >/dev/null 2>&1 || {
    echo "错误：未找到 GitHub CLI（gh）。请先安装并执行 gh auth login。" >&2
    exit 1
}

gh auth status >/dev/null 2>&1 || {
    echo "错误：GitHub CLI 尚未登录。请先执行 gh auth login。" >&2
    exit 1
}

if [[ -z "$run_id" ]]; then
    run_json="$(gh run list \
        --repo "$repo" \
        --workflow "$workflow" \
        --branch main \
        --status success \
        --limit 1 \
        --json databaseId,headSha,url)"

    # gh 的 Go template 会把 databaseId 当成浮点数，超过一定长度后可能输出
    # 科学计数法。运行 URL 中的 ID 是十进制字符串，因此从 URL 提取以保持精度。
    run_url="$(printf '%s\n' "$run_json" | sed -n 's/.*"url":"\([^"]*\)".*/\1/p')"
    head_sha="$(printf '%s\n' "$run_json" | sed -n 's/.*"headSha":"\([0-9a-fA-F]*\)".*/\1/p')"
    run_id="${run_url##*/}"

    if [[ -z "$run_url" || -z "$head_sha" || ! "$run_id" =~ ^[0-9]+$ ]]; then
        echo "错误：未找到或无法解析 main 分支最新成功的构建。" >&2
        exit 1
    fi
else
    run_line="$(gh run view "$run_id" \
        --repo "$repo" \
        --json status,conclusion,headSha,url \
        --template '{{.status}} {{.conclusion}} {{.headSha}} {{.url}}{{"\n"}}')"
    read -r status conclusion head_sha run_url <<<"$run_line"

    if [[ "$status" != "completed" || "$conclusion" != "success" ]]; then
        echo "错误：工作流 $run_id 尚未成功完成（$status/$conclusion）。" >&2
        exit 1
    fi
fi

short_sha="${head_sha:0:12}"
versioned_name="grok-fork-$short_sha"
versioned_path="$install_dir/$versioned_name"
active_path="$install_dir/grok"
backup_dir="$install_dir/backups"
state_dir="$backup_dir/fork-installs"
timestamp="$(date -u +%Y%m%d-%H%M%S)"
state_file="$state_dir/$timestamp.state"
download_dir="$(mktemp -d)"

cleanup() {
    rm -rf "$download_dir"
}
trap cleanup EXIT

echo "仓库：       $repo"
echo "工作流：     $run_id"
echo "提交：       $head_sha"
echo "运行页面：   $run_url"
echo "Artifact：   $artifact_name"

gh run download "$run_id" \
    --repo "$repo" \
    --name "$artifact_name" \
    --dir "$download_dir"

artifact_path="$(find "$download_dir" -type f \( -name 'grok-*-linux-x86_64' -o -name grok \) -print -quit)"
if [[ -z "$artifact_path" ]]; then
    echo "错误：Artifact 中未找到 Linux Grok 二进制。" >&2
    exit 1
fi
chmod +x "$artifact_path"

echo "验证下载的二进制："
"$artifact_path" --version

mkdir -p "$install_dir" "$state_dir"

previous_exists=0
previous_kind="missing"
previous_target=""
previous_backup=""

if [[ -L "$active_path" ]]; then
    previous_exists=1
    previous_kind="symlink"
    previous_target="$(readlink "$active_path")"
elif [[ -e "$active_path" ]]; then
    previous_exists=1
    previous_kind="file"
    previous_backup="$backup_dir/grok.before-fork-$timestamp"
    install -m 0755 "$active_path" "$previous_backup"
fi

# 先保存还原状态；替换失败时，原 active_path 仍保持不变。
cat >"$state_file" <<EOF
previous_exists=$previous_exists
previous_kind=$previous_kind
previous_target=$previous_target
previous_backup=$previous_backup
installed_name=$versioned_name
installed_path=$versioned_path
repo=$repo
run_id=$run_id
head_sha=$head_sha
EOF

install_tmp="$versioned_path.tmp.$$"
install -m 0755 "$artifact_path" "$install_tmp"
mv -f "$install_tmp" "$versioned_path"

link_tmp="$install_dir/.grok-link.$$"
ln -s "$versioned_name" "$link_tmp"
mv -Tf "$link_tmp" "$active_path"

echo
echo "替换完成："
echo "  当前链接： $active_path -> $(readlink "$active_path")"
echo "  二进制：   $versioned_path"
echo "  还原记录： $state_file"
echo
echo "当前版本："
"$active_path" --version
echo
echo "还原命令： ./scripts/restore-downloaded.sh"
echo "注意：已运行的 Grok 进程不会变化，请退出后重新启动。"
