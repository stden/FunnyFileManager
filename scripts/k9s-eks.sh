#!/usr/bin/env bash
#
# k9s + Amazon EKS helpers
#
# Source this file from your shell rc:
#   source /path/to/repo/scripts/k9s-eks.sh
#
# Requires: aws, k9s (and optionally kubectl, fzf)

_k9s_eks_err() {
  printf 'k9s-eks: %s\n' "$*" >&2
}

_k9s_eks_require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    _k9s_eks_err "missing required command: $1"
    return 127
  fi
}

_k9s_eks_default_region() {
  printf '%s' "${EKS_REGION:-${AWS_REGION:-${AWS_DEFAULT_REGION:-}}}"
}

_k9s_eks_default_profile() {
  printf '%s' "${EKS_PROFILE:-${AWS_PROFILE:-}}"
}

_k9s_eks_update_kubeconfig() {
  local cluster="$1"
  local region="$2"
  local profile="${3:-}"
  local alias="${4:-}"

  _k9s_eks_require_cmd aws || return

  if [[ -z "$cluster" ]]; then
    _k9s_eks_err "cluster name required"
    return 2
  fi

  if [[ -z "$region" ]]; then
    _k9s_eks_err "region required (pass as arg or set AWS_REGION/AWS_DEFAULT_REGION/EKS_REGION)"
    return 2
  fi

  local -a args=(eks update-kubeconfig --name "$cluster" --region "$region")
  if [[ -n "$profile" ]]; then
    args+=(--profile "$profile")
  fi
  if [[ -n "${KUBECONFIG:-}" ]]; then
    args+=(--kubeconfig "$KUBECONFIG")
  fi

  local output rc
  if [[ -n "$alias" ]]; then
    output="$(aws "${args[@]}" --alias "$alias" 2>&1)"
    rc=$?
    if [[ $rc -eq 0 ]]; then
      printf '%s\n' "$output" >&2
      printf '%s\n' "$alias"
      return 0
    fi

    # If --alias is not supported by the installed AWS CLI, fall back to parsing the created context.
    if [[ "$output" == *"Unknown options"*"--alias"* ]] || [[ "$output" == *"unknown option"*"--alias"* ]] || [[ "$output" == *"unrecognized arguments"*"--alias"* ]]; then
      output="$(aws "${args[@]}" 2>&1)"
      rc=$?
      printf '%s\n' "$output" >&2
      [[ $rc -eq 0 ]] || return $rc

      local context
      context="$(printf '%s\n' "$output" | sed -n 's/.*context \([^ ]\+\).*/\1/p' | head -n1)"
      if [[ -z "$context" ]]; then
        _k9s_eks_err "unable to determine kube context from aws output; pass an alias or use kubectl to select context"
        return 1
      fi
      printf '%s\n' "$context"
      return 0
    fi

    printf '%s\n' "$output" >&2
    return $rc
  fi

  output="$(aws "${args[@]}" 2>&1)"
  rc=$?
  printf '%s\n' "$output" >&2
  [[ $rc -eq 0 ]] || return $rc

  local context
  context="$(printf '%s\n' "$output" | sed -n 's/.*context \([^ ]\+\).*/\1/p' | head -n1)"
  if [[ -z "$context" ]]; then
    _k9s_eks_err "unable to determine kube context from aws output; set an alias or select the context manually"
    return 1
  fi
  printf '%s\n' "$context"
}

# List EKS clusters in a region.
# Usage: eks_ls [region] [profile]
eks_ls() {
  local region="${1:-$(_k9s_eks_default_region)}"
  local profile="${2:-$(_k9s_eks_default_profile)}"

  _k9s_eks_require_cmd aws || return
  if [[ -z "$region" ]]; then
    _k9s_eks_err "region required (pass as arg or set AWS_REGION/AWS_DEFAULT_REGION/EKS_REGION)"
    return 2
  fi

  local -a args=(eks list-clusters --region "$region" --output text --query 'clusters[]')
  if [[ -n "$profile" ]]; then
    args+=(--profile "$profile")
  fi

  aws "${args[@]}" | tr '\t' '\n'
}

# Update kubeconfig and switch kubectl context to the cluster (via alias by default).
# Usage: eks_use <cluster> [region] [profile] [alias]
eks_use() {
  local cluster="${1:-}"
  local region="${2:-$(_k9s_eks_default_region)}"
  local profile="${3:-$(_k9s_eks_default_profile)}"
  local alias="${4:-}"

  if [[ -z "$cluster" ]]; then
    _k9s_eks_err "Usage: eks_use <cluster> [region] [profile] [alias]"
    return 2
  fi

  if [[ -z "$alias" ]]; then
    alias="${cluster}@${region}"
  fi

  local context
  context="$(_k9s_eks_update_kubeconfig "$cluster" "$region" "$profile" "$alias")" || return

  _k9s_eks_require_cmd kubectl || return
  kubectl config use-context "$context"
}

# Update kubeconfig and launch k9s for the cluster.
# Usage: k9s_eks <cluster> [region] [profile] [alias]
k9s_eks() {
  local cluster="${1:-}"
  local region="${2:-$(_k9s_eks_default_region)}"
  local profile="${3:-$(_k9s_eks_default_profile)}"
  local alias="${4:-}"

  if [[ -z "$cluster" ]]; then
    _k9s_eks_err "Usage: k9s_eks <cluster> [region] [profile] [alias]"
    return 2
  fi

  if [[ -z "$alias" ]]; then
    alias="${cluster}@${region}"
  fi

  local context
  context="$(_k9s_eks_update_kubeconfig "$cluster" "$region" "$profile" "$alias")" || return

  _k9s_eks_require_cmd k9s || return
  k9s --context "$context"
}

# Fuzzy-pick an EKS cluster in a region and open k9s.
# Usage: k9s_eks_select [region] [profile]
k9s_eks_select() {
  local region="${1:-$(_k9s_eks_default_region)}"
  local profile="${2:-$(_k9s_eks_default_profile)}"

  _k9s_eks_require_cmd fzf || return

  local cluster
  cluster="$(eks_ls "$region" "$profile" | fzf --prompt 'EKS cluster> ')" || return
  k9s_eks "$cluster" "$region" "$profile"
}

# Print quick help.
k9s_eks_help() {
  cat <<'EOF'
k9s + EKS helper functions (source scripts/k9s-eks.sh)

  eks_ls [region] [profile]
  eks_use <cluster> [region] [profile] [alias]
  k9s_eks <cluster> [region] [profile] [alias]
  k9s_eks_select [region] [profile]   # requires fzf

Defaults:
  region:  EKS_REGION or AWS_REGION or AWS_DEFAULT_REGION
  profile: EKS_PROFILE or AWS_PROFILE

Example:
  k9s_eks my-cluster us-west-2 myprofile
EOF
}

