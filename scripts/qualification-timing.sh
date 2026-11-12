#!/usr/bin/env bash
# Source before command substitutions: fd 3 remains live when stdout is captured.
exec 3>&2
qualification_stage() {
  local label="$1" start=$SECONDS status=0
  shift
  printf 'QUALIFICATION_STAGE start label=%q utc=%s command=' "$label" "$(date -u +%FT%TZ)" >&3
  printf '%q ' "$@" >&3
  printf '\n' >&3
  # Do not open /dev/fd/3 with tee: on a redirected regular-file log that
  # reopens/truncates the file. Inherit the descriptor via cat instead.
  "$@" 2>&1 | tee >(cat >&3) || status=$?
  printf 'QUALIFICATION_STAGE end label=%q elapsed_seconds=%s exit=%s\n' \
    "$label" "$((SECONDS - start))" "$status" >&3
  return "$status"
}

cargo() {
  local args=() argument
  for argument in "$@"; do
    args+=("$argument")
    case "$argument" in
      build|test|check|clippy) args+=(-vv); break ;;
      +*) ;;
      *) qualification_stage cargo command cargo "$@"; return $? ;;
    esac
  done
  # Preserve every original argument, including the test-harness separator.
  local consumed=$((${#args[@]} - 1))
  qualification_stage cargo command cargo "${args[@]}" "${@:$((consumed + 1))}"
}