#!/usr/bin/env bash
set -euo pipefail

config_path="${1:-.github/config.json}"
repo="${GITHUB_REPOSITORY:-}"

if [[ -z "${repo}" ]]; then
    repo="$(gh repo view --json nameWithOwner --jq '.nameWithOwner')"
fi

if [[ -z "${repo}" ]]; then
    echo "Unable to determine GitHub repository. Set GITHUB_REPOSITORY=owner/name." >&2
    exit 2
fi

if [[ ! -f "${config_path}" ]]; then
    echo "GitHub config file not found: ${config_path}" >&2
    exit 2
fi

echo "Applying repository settings to ${repo}"
jq -c '.repository' "${config_path}" | gh api --method PATCH "repos/${repo}" --input -

echo "Applying Actions workflow permissions to ${repo}"
jq -c '.actions' "${config_path}" | gh api --method PUT "repos/${repo}/actions/permissions/workflow" --input -

for branch in $(jq -r '.branches | keys[]' "${config_path}"); do
    echo "Applying branch protection to ${repo}:${branch}"
    jq -c --arg branch "${branch}" '.branches[$branch].protection' "${config_path}" |
        gh api --method PUT "repos/${repo}/branches/${branch}/protection" --input -
done
