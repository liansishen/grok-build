#!/usr/bin/env bash
# 还原 download-replace.sh 替换前的 Grok。
#
# 用法：
#   ./scripts/restore-downloaded.sh
#   ./scripts/restore-downloaded.sh --list
#   ./scripts/restore-downloaded.sh --state ~/.grok/bin/backups/fork-installs/<记录>.state
#
# 环境变量：
#   GROK_INSTALL_DIR    安装目录，默认 ~/.grok/bin

set -Eeuo pipefail

install_dir="${GROK_INSTALL_DIR:-$HOME/.grok/bin}"
active_path="$install_dir/grok"
state_dir="$install_dir/backups/fork-installs"
state_file=""
list_only=0

usage() {
    cat <<'EOF'
还原 download-replace.sh 替换前的 Grok。

用法：
  ./scripts/restore-downloaded.sh [选项]

选项：
  --list          列出可用的还原记录
  --state FILE    使用指定的还原记录；默认使用最新记录
  -h, --help      显示帮助
EOF
}

while (($#)); do
    case "$1" in
        --list)
            list_only=1
            shift
            ;;
        --state)
            [[ $# -ge 2 ]] || { echo "错误：--state 缺少参数" >&2; exit 2; }
            state_file="$2"
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

if ((list_only)); then
    if [[ ! -d "$state_dir" ]]; then
        echo "没有还原记录：$state_dir"
        exit 0
    fi

    mapfile -t states < <(find "$state_dir" -maxdepth 1 -type f -name '*.state' -printf '%T@ %p\n' | sort -nr | cut -d' ' -f2-)
    if ((${#states[@]} == 0)); then
        echo "没有还原记录：$state_dir"
        exit 0
    fi

    echo "可用还原记录（最新在前）："
    for item in "${states[@]}"; do
        printf '  %s\n' "$item"
    done
    exit 0
fi

if [[ -z "$state_file" ]]; then
    if [[ ! -d "$state_dir" ]]; then
        echo "错误：没有还原记录目录：$state_dir" >&2
        exit 1
    fi
    state_file="$(find "$state_dir" -maxdepth 1 -type f -name '*.state' -printf '%T@ %p\n' | sort -nr | head -n1 | cut -d' ' -f2-)"
fi

if [[ -z "$state_file" || ! -f "$state_file" ]]; then
    echo "错误：找不到还原记录：${state_file:-（空）}" >&2
    exit 1
fi

# 状态文件由配套脚本生成。逐行解析而不是 source，避免执行其中的内容。
previous_exists=""
previous_kind=""
previous_target=""
previous_backup=""
installed_name=""
installed_path=""
repo=""
run_id=""
head_sha=""
while IFS='=' read -r key value; do
    case "$key" in
        previous_exists) previous_exists="$value" ;;
        previous_kind) previous_kind="$value" ;;
        previous_target) previous_target="$value" ;;
        previous_backup) previous_backup="$value" ;;
        installed_name) installed_name="$value" ;;
        installed_path) installed_path="$value" ;;
        repo) repo="$value" ;;
        run_id) run_id="$value" ;;
        head_sha) head_sha="$value" ;;
    esac
done <"$state_file"

if [[ "$previous_exists" != "0" && "$previous_exists" != "1" ]]; then
    echo "错误：还原记录格式无效：$state_file" >&2
    exit 1
fi

# 防止误用其他目录的状态记录。
expected_prefix="$install_dir/"
if [[ -n "$installed_path" && "$installed_path" != "$expected_prefix"* ]]; then
    echo "错误：记录中的安装路径不属于当前 GROK_INSTALL_DIR：$installed_path" >&2
    exit 1
fi

restore_tmp="$install_dir/.grok-restore.$$"
rm -f "$restore_tmp"

case "$previous_kind" in
    symlink)
        [[ -n "$previous_target" ]] || {
            echo "错误：记录缺少原符号链接目标。" >&2
            exit 1
        }
        if [[ "$previous_target" = /* ]]; then
            resolved_target="$previous_target"
        else
            resolved_target="$install_dir/$previous_target"
        fi
        [[ -e "$resolved_target" ]] || {
            echo "错误：原二进制已不存在：$resolved_target" >&2
            exit 1
        }
        ln -s "$previous_target" "$restore_tmp"
        mv -Tf "$restore_tmp" "$active_path"
        ;;
    file)
        [[ -n "$previous_backup" && -f "$previous_backup" ]] || {
            echo "错误：原文件备份不存在：${previous_backup:-（空）}" >&2
            exit 1
        }
        install -m 0755 "$previous_backup" "$restore_tmp"
        mv -Tf "$restore_tmp" "$active_path"
        ;;
    missing)
        if [[ "$previous_exists" != "0" ]]; then
            echo "错误：还原记录状态不一致。" >&2
            exit 1
        fi
        rm -f "$active_path"
        ;;
    *)
        echo "错误：未知的原安装类型：${previous_kind:-（空）}" >&2
        exit 1
        ;;
esac

echo "已使用记录还原：$state_file"
if [[ -e "$active_path" || -L "$active_path" ]]; then
    if [[ -L "$active_path" ]]; then
        echo "当前链接：$active_path -> $(readlink "$active_path")"
    else
        echo "当前文件：$active_path"
    fi
    echo "当前版本："
    "$active_path" --version
else
    echo "替换前不存在 $active_path，已将其移除。"
fi

# 成功后消费记录，确保连续执行 restore 会回退不同的安装批次，而不是重复同一记录。
rm -f "$state_file"

echo "注意：已运行的 Grok 进程不会变化，请退出后重新启动。"
