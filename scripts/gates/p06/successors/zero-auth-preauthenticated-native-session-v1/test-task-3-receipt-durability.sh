#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/../../../../.." && pwd -P)
scratch=$(mktemp -d "${TMPDIR:-/tmp}/taskseal-p06-zero-auth-task3-descendant.XXXXXX")
trap 'rm -rf "$scratch"' EXIT HUP INT TERM

git clone -q --no-hardlinks "$root" "$scratch/repository"
git -C "$scratch/repository" config user.name taskseal-receipt-fixture
git -C "$scratch/repository" config user.email taskseal-receipt-fixture@invalid
git -C "$scratch/repository" commit -q --allow-empty -m 'fixture: descendant after task 3 receipt'

actual=$("$scratch/repository/scripts/gates/p06/successors/zero-auth-preauthenticated-native-session-v1/test-task-3-receipt.sh" 2>&1)
test "$actual" = P06_ZERO_AUTH_TASK_3_RECEIPT_PASS

printf '%s\n' P06_ZERO_AUTH_TASK_3_RECEIPT_DURABILITY_PASS
