# Shared by bin/start, bin/stop, bin/status. Sourced, not executed.

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUN_DIR="$ROOT/.run"
LOG_DIR="$RUN_DIR/logs"

INFERENCE_ADDR="${ET_INFERENCE_ADDR:-127.0.0.1:50051}"
SERVER_ADDR="${ET_SERVER_ADDR:-127.0.0.1:8080}"
WEB_PORT="${ET_WEB_PORT:-5173}"

SERVICES=(inference server web)

pid_file() { echo "$RUN_DIR/$1.pid"; }
log_file() { echo "$LOG_DIR/$1.log"; }

# Prints the PID if the service's recorded process is alive.
alive_pid() {
    local f; f="$(pid_file "$1")"
    [[ -f "$f" ]] || return 1
    local pid; pid="$(cat "$f")"
    [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null && echo "$pid"
}

# Newest .onnx under models/ by modification time.
latest_model() {
    ls -t "$ROOT"/models/*.onnx 2>/dev/null | head -n 1
}

info()  { printf '\033[1;34m==>\033[0m %s\n' "$*"; }
warn()  { printf '\033[1;33mwarning:\033[0m %s\n' "$*" >&2; }
die()   { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }
