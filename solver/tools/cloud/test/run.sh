#!/usr/bin/env bash
# Tests of the unattended-run scripts that need no cloud: chain.sh with the wall and the ledger (run.sh), wall-node.sh
# and wall.sh, against a stand-in for the aws CLI (test/bin/aws: S3 as a directory, the group as a file), for ssh
# (test/bin/ssh: the machine is this one) and for sudo and shutdown.  The scenarios that run the chain (chain, ledger, node, sweep) need the release build of the solver
# (cd solver/frame && cargo build --release; or TAXMAN_BIN names another build of the same sources, say the debug build
# that `cargo build` makes) and rsync; launch and status need neither.  sweep.sh's scenario also runs the real verify.sh
# on the sweep's files, which replays the committed games three times.
#
#   solver/tools/cloud/test/run.sh [SCENARIO...]   all of them, or the named (chain, ledger, node, launch, status, sweep)
#
# Prints `ok` or `FAIL` for each check and exits with the number of failures.  A few minutes: the chains are small.

set -u

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
real_cloud="$(cd "$here/.." && pwd)"
crate="$(cd "$real_cloud/../../frame" && pwd)"
export TAXMAN_BIN="${TAXMAN_BIN:-$crate/target/release/taxman}"
selected=("$@")
[ "${#selected[@]}" -gt 0 ] || selected=(chain ledger node launch status sweep)
for s in "${selected[@]}"; do
    case "$s" in
        chain | ledger | node | sweep)
            [ -x "$TAXMAN_BIN" ] || {
                echo "build the solver first: cd $crate && cargo build --release" >&2
                exit 2
            }
            command -v rsync >/dev/null 2>&1 || {
                echo "rsync is needed (the stand-in for aws s3 sync)" >&2
                exit 2
            }
            ;;
    esac
done

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"; kill $(jobs -p) 2>/dev/null' EXIT
# The scripts under test are a copy of the directory's, but for verify.sh, which replays the committed games
# (half a minute) every time a chain completes.
cloud="$tmp/cloud"
mkdir -p "$cloud"
cp "$real_cloud"/*.sh "$real_cloud/wall-policy.json" "$cloud/"
printf '#!/usr/bin/env bash\necho "verify.sh (a stand-in): $*"\n' >"$cloud/verify.sh"
cp "$cloud/verify.sh" "$tmp/verify.sh.stub"
# ... and setup.sh, which installs and builds: the stand-in records how it was called and may sleep a while.
printf '%s\n' '#!/usr/bin/env bash' \
    'echo "setup.sh: NO_PULL=${NO_PULL:-} SKIP_TESTS=${SKIP_TESTS:-} BUILD=${BUILD-unset}" \
        "RELEASE_TESTS=${RELEASE_TESTS:-}" >>"$TAXMAN_WORK/setup-stub.log"' \
    '[ -z "${STUB_SETUP_SLEEP:-}" ] || sleep "$STUB_SETUP_SLEEP"' \
    '[ -z "${STUB_SETUP_FAIL:-}" ] || exit 1' >"$cloud/setup.sh"
chmod +x "$cloud"/*.sh
export TAXMAN_ROOT="$(cd "$crate/../.." && pwd)"
export PATH="$here/bin:$PATH"
export FAKE_S3="$tmp/s3" FAKE_STATE="$tmp/state" FAKE_LOG="$tmp/aws.log"
export AWS_DEFAULT_REGION=us-east-1
mkdir -p "$FAKE_S3" "$FAKE_STATE"
failures=0

check() {
    local what="$1"
    shift
    if "$@"; then
        printf 'ok   %s\n' "$what"
    else
        printf 'FAIL %s\n' "$what"
        failures=$((failures + 1))
    fi
}

# The object NAME of the fake bucket b.
obj() {
    printf '%s/b/%s\n' "$FAKE_S3" "$1"
}

# Whether the file $1 holds the line (fixed text) $2 / the pattern $2.
has_line() { grep -qxF -- "$2" "$1" 2>/dev/null; }
matches() { grep -q -- "$2" "$1" 2>/dev/null; }
lacks() { [ -e "$1" ] && ! grep -q -- "$2" "$1"; }
exists() { [ -e "$1" ]; }
absent() { [ ! -e "$1" ]; }
equals() { [ "$1" = "$2" ]; }
# Whether the directory $1 has a file whose name matches $2.
lists() { ls "$1" 2>/dev/null | grep -q -- "$2"; }

# The seconds the fake machine has been up and its boot, which run.sh reads from these.
machine() {
    export UPTIME_SECONDS="$1" BOOT_ID="$2"
}

wait_for() {
    local tries=0
    until "$@"; do
        tries=$((tries + 1))
        [ "$tries" -le 600 ] || return 1
        sleep 0.2
    done
}

# --- chain.sh: the wall, the date --------------------------------------------------------------------------------

scenario_chain() {
    echo "== chain.sh: the wall and the date"
    local w="$tmp/chain" n
    # A wall of 1.8 ms: the first game that the head searches for longer stops the chain.
    (
        export TAXMAN_WORK="$w" GAME_HOURS=0.0000005 THREADS=2 PROGRESS=0
        "$cloud/chain.sh" 1000 >"$tmp/chain.out" 2>&1
        echo $? >"$tmp/chain.status"
    )
    check "the wall ends the chain with status 4" has_line "$tmp/chain.status" 4
    check "the log says wall at N after H hours" matches "$w/logs/chain.log" 'wall at [0-9]* after [0-9.]* hours'
    check "the outcome says so" matches "$w/chain.outcome" '^wall at [0-9]* after [0-9.]* hours$'
    n="$(sed -n 's/^wall at \([0-9]*\) after.*/\1/p' "$w/chain.outcome")"
    check "the game has a checkpoint and a clock" exists "$w/ckpt/n$(printf %04d "$n").clock"
    check "and no line in the results" lacks "$w/results/strict.jsonl" "\"n\":$n,"
    # With a larger limit it goes on from its checkpoint and its clock, and completes.
    (
        export TAXMAN_WORK="$w" GAME_HOURS=1 THREADS=2 PROGRESS=0
        "$cloud/chain.sh" $((n + 1)) >"$tmp/chain2.out" 2>&1
        echo $? >"$tmp/chain2.status"
    )
    check "a larger limit completes the chain" has_line "$tmp/chain2.status" 0
    check "and the clock is gone" absent "$w/ckpt/n$(printf %04d "$n").clock"
    check "and the game is in the results" matches "$w/results/strict.jsonl" "\"n\":$n,"

    # A date already within the margin: the deadline has passed before the solver starts.
    (
        export TAXMAN_WORK="$tmp/chain-date" RUN_UNTIL=$(($(date +%s) + 100)) THREADS=2 PROGRESS=0
        "$cloud/chain.sh" 50 >"$tmp/chain3.out" 2>&1
        echo $? >"$tmp/chain3.status"
    )
    check "a date within the margin ends the chain with the deadline's status 3" has_line "$tmp/chain3.status" 3
    check "and the outcome names the date" matches "$tmp/chain-date/chain.outcome" '^deadline: the date 20'
}

# --- the ledger: the budget across machines ----------------------------------------------------------------------

scenario_ledger() {
    echo "== run.sh: the ledger in S3"
    local ledger
    ledger="$(obj t1/run/ledger)"
    : >"$FAKE_LOG"
    # The first machine, up half an hour: base 0.
    (
        machine 1800 boot-a
        export TAXMAN_WORK="$tmp/m1" S3_DEST=s3://b/t1 BUDGET_HOURS=10 THREADS=2 PROGRESS=0
        "$cloud/chain.sh" 60 >"$tmp/m1.out" 2>&1
        echo $? >"$tmp/m1.status"
    )
    check "the first machine completes the chain" has_line "$tmp/m1.status" 0
    check "the ledger has the run's start" matches "$ledger" '^start=[0-9]\{10\}$'
    check "and its boot" has_line "$ledger" boot=boot-a
    check "and base 0 and half an hour" has_line "$ledger" base=0
    check "and its uptime in hours" has_line "$ledger" hours=0.5000
    check "the results are in S3" matches "$(obj t1/results/strict.jsonl)" '"n":60,'
    local start
    start="$(sed -n 's/^start=//p' "$ledger")"

    # A replacement machine, up ten minutes: the first one's half hour is its base.
    (
        machine 600 boot-b
        export TAXMAN_WORK="$tmp/m2" S3_DEST=s3://b/t1 BUDGET_HOURS=10 THREADS=2 PROGRESS=0
        mkdir -p "$tmp/m2"
        "$cloud/save.sh" --restore >/dev/null 2>&1
        "$cloud/chain.sh" 60 >"$tmp/m2.out" 2>&1
        echo $? >"$tmp/m2.status"
    )
    check "the replacement finds the chain done" has_line "$tmp/m2.status" 0
    check "the run's start is kept" has_line "$ledger" "start=$start"
    check "the replacement's base is the first machine's hours" has_line "$ledger" base=0.5000
    check "and the hours are the base and its own" has_line "$ledger" hours=0.6667

    # The same machine again (the same boot): its hours are not counted twice.
    (
        machine 3600 boot-b
        export TAXMAN_WORK="$tmp/m2" S3_DEST=s3://b/t1 BUDGET_HOURS=10 THREADS=2 PROGRESS=0
        "$cloud/chain.sh" 60 >"$tmp/m2b.out" 2>&1
    )
    check "a second script of the same boot keeps the base" has_line "$ledger" base=0.5000
    check "and the hours are base plus the boot's uptime" has_line "$ledger" hours=1.5000

    # A budget the earlier machines used up: nothing is started.
    (
        machine 600 boot-c
        export TAXMAN_WORK="$tmp/m3" S3_DEST=s3://b/t1 BUDGET_HOURS=1.5 THREADS=2 PROGRESS=0
        mkdir -p "$tmp/m3"
        "$cloud/save.sh" --restore >/dev/null 2>&1
        "$cloud/chain.sh" 60 >"$tmp/m3.out" 2>&1
        echo $? >"$tmp/m3.status"
    )
    check "a spent budget ends the chain with status 3" has_line "$tmp/m3.status" 3
    check "the outcome names the budget" matches "$tmp/m3/chain.outcome" '^deadline: the budget of 1.5 instance-hours$'
    check "the log says the deadline had passed" matches "$tmp/m3/logs/chain.log" \
        'the deadline has passed: the budget of 1.5 instance-hours'
    check "and the solver did not run" absent "$tmp/m3/logs/chain.out"

    # A ledger that cannot be read stops the machine: it must not run uncounted.
    (
        machine 600 boot-d
        export TAXMAN_WORK="$tmp/m4" S3_DEST=s3://b/t1 BUDGET_HOURS=10 FAKE_FAIL=nothing THREADS=2
        printf 'start=soon\nhours=lots\n' >"$ledger"
        "$cloud/chain.sh" 60 >"$tmp/m4.out" 2>&1
        echo $? >"$tmp/m4.status"
    )
    check "a ledger with no numbers stops the machine" has_line "$tmp/m4.status" 1
    check "and says so" matches "$tmp/m4.out" 'has no number for'
}

# --- wall-node.sh ------------------------------------------------------------------------------------------------

# A fresh node: its work directory, the config put in the fake S3 under t-NAME.
node_config() {
    local name="$1"
    shift
    mkdir -p "$(obj "$name/run")"
    printf '%s\n' "COMMIT=0123456789012345678901234567890123456789" "INSTANCE_TYPE=c8g.large" "WINDOW=2" "THREADS=2" \
        "CHECKPOINT_INTERVAL=2" "$@" >"$(obj "$name/run/config")"
}

# Runs a node for the run NAME in its own work directory, to its end.
node() {
    local name="$1"
    shift
    (
        export TAXMAN_WORK="$tmp/n-$name" S3_DEST="s3://b/$name" ASG=taxman-wall SKIP_SETUP=1 RETRY_SLEEP=0 \
            STOP_POLL=1 PROGRESS=0 "$@"
        machine 100 "boot-$name-$RANDOM"
        "$cloud/wall-node.sh" >"$tmp/n-$name.out" 2>&1
    )
}

scenario_node() {
    echo "== wall-node.sh"
    local name

    # A chain that completes: the run ends, the group goes to 0, the machine powers off.
    name=done
    node_config $name "CHAIN_TO=40" "BUDGET_HOURS=10" "RUN_UNTIL=$(($(date +%s) + 86400))"
    : >"$FAKE_LOG"
    echo 1 >"$FAKE_STATE/desired"
    node $name
    check "a completed chain: the end is recorded" matches "$(obj $name/run/ended)" '^reason=complete: the range 1-40$'
    check "the group is taken to capacity 0" has_line "$FAKE_STATE/desired" 0
    check "the machine powers off" matches "$FAKE_LOG" '^shutdown -h now$'
    check "the machine recorded itself" exists "$(ls -d "$(obj $name/run/boots)"/* | head -n 1)"
    check "the results are in S3" matches "$(obj $name/results/strict.jsonl)" '"n":40,'
    check "the logs are in S3" exists "$(obj $name/logs/wall-node.log)"
    check "the ledger is there" matches "$(obj $name/run/ledger)" '^hours=0.0278$'

    # An ended run: a machine that starts later only takes the group to 0 again and powers off, with no chain.
    : >"$FAKE_LOG"
    echo 1 >"$FAKE_STATE/desired"
    node $name
    check "an ended run is not taken up again" lacks "$tmp/n-$name.out" 'chain.sh [0-9]'
    check "but the group is taken to 0 again" has_line "$FAKE_STATE/desired" 0
    check "and the machine powers off" matches "$FAKE_LOG" '^shutdown -h now$'
    check "the first end stands" matches "$(obj $name/run/ended)" '^reason=complete: the range 1-40$'

    # The wall: a game over the limit ends the run with its reason, whatever the machine does next.
    name=wall
    node_config $name "CHAIN_TO=1000" "GAME_HOURS=0.0000005" "BUDGET_HOURS=10"
    : >"$FAKE_LOG"
    echo 1 >"$FAKE_STATE/desired"
    node $name
    check "the wall: the end names the game" matches "$(obj $name/run/ended)" \
        '^reason=wall at [0-9]* after [0-9.]* hours$'
    check "the group is taken to 0" has_line "$FAKE_STATE/desired" 0
    check "the game's checkpoint and clock were saved to S3" lists "$(obj $name/ckpt)" 'clock$'

    # The budget: the chain is stopped by the deadline the ledger makes, and the end says the budget.
    name=budget
    node_config $name "CHAIN_TO=1000" "BUDGET_HOURS=0.0001" "LEDGER_MARGIN=0"
    echo 1 >"$FAKE_STATE/desired"
    node $name
    check "the budget: the end names it" matches "$(obj $name/run/ended)" \
        '^reason=deadline: the budget of 0.0001 instance-hours$'

    # More machines than MAX_INSTANCES: a boot loop ends the run.
    name=loop
    node_config $name "CHAIN_TO=40" "MAX_INSTANCES=2"
    mkdir -p "$(obj $name/run/boots)"
    touch "$(obj $name/run/boots)/1" "$(obj $name/run/boots)/2"
    echo 1 >"$FAKE_STATE/desired"
    node $name
    check "a third machine ends the run as a boot loop" matches "$(obj $name/run/ended)" 'MAX_INSTANCES=2'
    check "without a chain" absent "$tmp/n-$name/results/strict.jsonl"

    # A group that cannot be set to 0: the end is still recorded, the tries are made, the machine powers off.
    name=denied
    node_config $name "CHAIN_TO=20"
    : >"$FAKE_LOG"
    echo 1 >"$FAKE_STATE/desired"
    node $name FAKE_FAIL=set-desired-capacity
    check "a denied capacity: the end is recorded" matches "$(obj $name/run/ended)" '^reason=complete'
    check "five tries are made" equals "$(grep -c 'set-desired-capacity' "$FAKE_LOG")" 5
    check "and the machine powers off" matches "$FAKE_LOG" '^shutdown -h now$'

    # The setup is run as a build and no tests, with the pinned checkout left as it is.
    name=setup
    node_config $name "CHAIN_TO=20"
    echo 1 >"$FAKE_STATE/desired"
    node $name SKIP_SETUP=
    check "setup.sh is called with NO_PULL and SKIP_TESTS" matches "$tmp/n-$name/setup-stub.log" \
        'setup.sh: NO_PULL=1 SKIP_TESTS=1'
    check "and builds the solver alone" matches "$tmp/n-$name/setup-stub.log" 'BUILD=--bin taxman RELEASE_TESTS=$'

    # What the environment says about the build goes on to setup.sh; a setup that fails ends the run with the error.
    name=setupenv
    node_config $name "CHAIN_TO=20" "RELEASE_TESTS=1"
    echo 1 >"$FAKE_STATE/desired"
    node $name SKIP_SETUP= BUILD="--bin taxman --example ledger" STUB_SETUP_FAIL=1
    check "BUILD, and RELEASE_TESTS from the config, reach setup.sh" matches "$tmp/n-$name/setup-stub.log" \
        'BUILD=--bin taxman --example ledger RELEASE_TESTS=1$'
    check "a setup that fails ends the run" matches "$(obj $name/run/ended)" '^reason=setup.sh failed'
    check "and the chain does not start" absent "$tmp/n-$name/results/strict.jsonl"

    # A signal in the setup (the shutdown of a spot interruption sends SIGTERM to every process) is not the run's end:
    # no end is recorded, the group is left alone, and the machine does not power itself off.
    name=sigsetup
    node_config $name "CHAIN_TO=20"
    echo 1 >"$FAKE_STATE/desired"
    : >"$FAKE_LOG"
    node $name SKIP_SETUP= STUB_SETUP_SLEEP=30 &
    node_pid=$!
    wait_for matches "$tmp/n-$name/logs/wall-node.log" 'Z setup.sh$' || echo "the setup did not start"
    # As a shutdown does: the signal reaches the node and what it is running.
    local node_script
    node_script="$(pgrep -f "$cloud/wall-node.sh")"
    kill -TERM $node_script
    pkill -TERM -P "$node_script"
    wait "$node_pid" 2>/dev/null
    check "a signal in the setup does not end the run" absent "$(obj $name/run/ended)"
    check "and leaves the group alone" has_line "$FAKE_STATE/desired" 1
    check "and the machine does not power itself off" lacks "$FAKE_LOG" '^shutdown'
    check "the node says it was signalled" matches "$tmp/n-$name/logs/wall-node.log" 'signalled: leaving the run'

    # A node that cannot read its config ends the run with the error.
    name=noconfig
    mkdir -p "$(obj $name)"
    echo 1 >"$FAKE_STATE/desired"
    node $name
    check "no config: the end is recorded" matches "$(obj $name/run/ended)" 'cannot read the run.s config'

    # The stop object: the chain is stopped cleanly and the run ends on request.
    name=stop
    node_config $name "CHAIN_TO=1300" "EXTRA_ARGS=--tactic gf2=off --tactic lagrange=on --tactic symmetry=off"
    echo 1 >"$FAKE_STATE/desired"
    node $name &
    local node_pid=$!
    wait_for exists "$tmp/n-$name/chain.pid" || echo "no chain.pid"
    sleep 3
    printf 'stop\n' >"$(obj $name/run/stop)"
    wait "$node_pid"
    check "a stop request ends the run" matches "$(obj $name/run/ended)" '^reason=stopped on request'
    check "the group is taken to 0" has_line "$FAKE_STATE/desired" 0
    check "the chain left a checkpoint" lists "$(obj $name/ckpt)" 'ckpt$'

    # An interruption: the chain is stopped by a signal and there is no stop object.  The machine waits to be taken
    # away; here it is (killed in its wait), and nothing has ended the run.
    name=interrupt
    node_config $name "CHAIN_TO=1300" "EXTRA_ARGS=--tactic gf2=off --tactic lagrange=on --tactic symmetry=off"
    echo 1 >"$FAKE_STATE/desired"
    node $name INTERRUPTION_WAIT=60 &
    node_pid=$!
    wait_for exists "$tmp/n-$name/chain.pid" || echo "no chain.pid"
    sleep 3
    kill -TERM "$(cat "$tmp/n-$name/chain.pid")"
    wait_for matches "$tmp/n-$name/logs/wall-node.log" 'an interruption, it seems' || echo "no interruption seen"
    pkill -P "$node_pid" -f 'sleep 60' 2>/dev/null
    kill -KILL "$node_pid" 2>/dev/null
    wait "$node_pid" 2>/dev/null
    check "an interruption does not end the run" absent "$(obj $name/run/ended)"
    check "and leaves the group alone" has_line "$FAKE_STATE/desired" 1
    check "and the chain's checkpoint is kept for the replacement" lists "$(obj $name/ckpt)" 'ckpt$'
    check "and its state is saved" exists "$(obj $name/results/strict.jsonl)"
}

# --- wall.sh -----------------------------------------------------------------------------------------------------

scenario_launch() {
    echo "== wall.sh"
    local commit=0123456789abcdef0123456789abcdef01234567 wall="$cloud/wall.sh"
    export BUCKET=taxman-013764204840-us-east-1 RUN=test WALL_DIR="$tmp/wall"
    # The object NAME of the run's bucket.
    wobj() { printf '%s/%s/%s\n' "$FAKE_S3" "$BUCKET" "$1"; }
    # The user data the template carries, decoded.
    template_user_data() {
        python3 -c 'import json, sys, base64
print(base64.b64decode(json.load(open(sys.argv[1]))["UserData"]).decode(), end="")' "$FAKE_STATE/template"
    }
    # The calls of the launch that change the account, in the order made, as `service command`.
    changes_in_order() {
        grep 'put-role-policy\|create-auto-scaling-group\|put-scheduled-update-group-action\|set-desired-capacity' \
            "$FAKE_LOG" | awk '{printf "%s %s,", $1, $2}'
    }
    rm -rf "$FAKE_STATE"
    : >"$FAKE_LOG"

    DRY_RUN=1 "$wall" launch --commit "$commit" --until 2099-01-01 >"$tmp/dry.out" 2>&1
    check "a dry run succeeds" equals $? 0
    check "and changes nothing" absent "$FAKE_STATE/asg"
    check "it prints the group's creation" matches "$tmp/dry.out" '^+ aws autoscaling create-auto-scaling-group'
    check "with capacity rebalancing off" matches "$tmp/dry.out" 'no-capacity-rebalance'
    check "it says it did not run it" matches "$tmp/dry.out" 'dry run: not run'
    check "the zones are by price, the cheapest first (b is 0.6603, a 0.6759, c has none)" matches "$tmp/dry.out" \
        'zones: us-east-1b us-east-1a us-east-1c'
    # The user data it prints is a script.
    sed -n '/^--- the user data of the launch template ---$/,/^--- the config of the run/p' "$tmp/dry.out" |
        sed '1d;$d' >"$tmp/user-data.sh"
    check "the user data is valid bash" bash -n "$tmp/user-data.sh"
    check "it checks out the commit" matches "$tmp/user-data.sh" "git -C /home/ec2-user/taxman checkout $commit"
    check "it takes wall-node.sh from solver/ if the checkout has it" matches "$tmp/user-data.sh" \
        '^node=solver/tools/cloud/wall-node.sh$'
    check "else from rust/, where commits before the restructure have it" matches "$tmp/user-data.sh" \
        '|| node=rust/tools/cloud/wall-node.sh$'
    check "else it ends the run" matches "$tmp/user-data.sh" 'fail "wall-node.sh is in neither'
    check "it starts the script it found in tmux" matches "$tmp/user-data.sh" 'tmux new-session -d -s tx .* "\$node"'
    check "it checks that the session lives a few seconds later" matches "$tmp/user-data.sh" \
        'tmux has-session -t tx || fail "wall-node did not start"'
    check "it sets the timer from the budget" matches "$tmp/user-data.sh" '^shutdown -h +4560$'
    check "the config has no build cache setting by default" lacks "$tmp/dry.out" '^RELEASE_TESTS='
    RELEASE_TESTS=1 REQUIRE_CACHED=1 DRY_RUN=1 "$wall" launch --commit "$commit" --until 2099-01-01 \
        >"$tmp/dry-cache.out" 2>&1
    check "RELEASE_TESTS and REQUIRE_CACHED go into the config" matches "$tmp/dry-cache.out" '^RELEASE_TESTS=1$'
    check "both" matches "$tmp/dry-cache.out" '^REQUIRE_CACHED=1$'
    check "the commands that only read are not printed" lacks "$tmp/dry.out" '^+ aws ec2 describe-vpcs'
    DRY_RUN=1 "$wall" launch --verbose --commit "$commit" --until 2099-01-01 >"$tmp/dry-verbose.out" 2>&1
    check "with --verbose they are" matches "$tmp/dry-verbose.out" '^+ aws ec2 describe-vpcs'

    "$wall" launch --commit "$commit" --until 2099-01-01 --zones "us-east-1b us-east-1a" >"$tmp/launch.out" 2>&1
    check "a launch succeeds" equals $? 0
    check "the group has capacity 1 at the end" has_line "$FAKE_STATE/desired" 1
    check "its mixed policy is spot, the lowest price" matches "$FAKE_STATE/asg" \
        '"SpotAllocationStrategy":"lowest-price"'
    check "the scheduled action is the date plus two hours" has_line "$FAKE_STATE/scheduled" 2099-01-01T02:00:00Z
    check "the template is JSON" python3 -c 'import json,sys; json.load(open(sys.argv[1]))' "$FAKE_STATE/template"
    check "the template's user data is the script" equals "$(template_user_data)" "$(cat "$tmp/user-data.sh")"
    check "the config is in S3" matches "$(wobj wall-test/run/config)" '^BUDGET_HOURS=74$'
    check "with the date as seconds" matches "$(wobj wall-test/run/config)" '^RUN_UNTIL=40[0-9]*$'
    check "the policy is on the role" exists "$FAKE_STATE/role-policy"
    check "the order: policy, group, schedule, start" equals "$(changes_in_order)" \
        "iam put-role-policy,autoscaling create-auto-scaling-group,autoscaling put-scheduled-update-group-action,autoscaling set-desired-capacity,"
    "$wall" launch --commit "$commit" --until 2099-01-01 >"$tmp/again.out" 2>&1
    check "a second launch is refused while the group exists" matches "$tmp/again.out" 'the group taxman-wall exists'
    "$wall" launch --commit abc --until 2099-01-01 >"$tmp/bad.out" 2>&1
    check "a short commit is refused" matches "$tmp/bad.out" '40 hex digits'

    "$wall" status >"$tmp/status.out" 2>&1
    check "status reads the run" matches "$tmp/status.out" 'budget 74 instance-hours'
    check "and says it has not ended" matches "$tmp/status.out" 'not ended'

    "$wall" stop >"$tmp/stop.out" 2>&1
    check "stop puts the stop object" exists "$(wobj wall-test/run/stop)"
    "$wall" stop --now >"$tmp/stopnow.out" 2>&1
    check "stop --now records the end" matches "$(wobj wall-test/run/ended)" 'stopped at once'
    check "and takes the group to 0" has_line "$FAKE_STATE/desired" 0
    "$wall" status >"$tmp/status2.out" 2>&1
    check "status shows the end" matches "$tmp/status2.out" 'ENDED .*stopped at once'

    "$wall" cleanup >"$tmp/cleanup.out" 2>&1
    check "cleanup succeeds on an ended run" equals $? 0
    check "its end checks are printed with their commands" matches "$tmp/cleanup.out" '^+ aws ec2 describe-volumes'
    check "the group is gone" absent "$FAKE_STATE/asg"
    check "the template is gone" absent "$FAKE_STATE/template"
    check "the security group is gone" absent "$FAKE_STATE/sg"
    check "the policy is gone" absent "$FAKE_STATE/role-policy"
    check "the run's state stays in S3" exists "$(wobj wall-test/run/config)"

    "$wall" launch --commit "$commit" --until 2099-01-01 >"$tmp/relaunch.out" 2>&1
    check "a launch over an existing run's state is refused" matches "$tmp/relaunch.out" 'run/config exists'
    mkdir -p "$(wobj wall-test/run/boots)"
    touch "$(wobj wall-test/run/boots)/1"
    "$wall" launch --commit "$commit" --until 2099-01-01 --continue >"$tmp/continue.out" 2>&1
    check "--continue goes on with it" equals $? 0
    check "and removes the end" absent "$(wobj wall-test/run/ended)"
    check "and the count of the machines so far" absent "$(wobj wall-test/run/boots)/1"
    check "and the stop" absent "$(wobj wall-test/run/stop)"
    "$wall" cleanup >"$tmp/cleanup2.out" 2>&1
    check "cleanup refuses a run that has not ended" matches "$tmp/cleanup2.out" 'has not ended'
    "$wall" cleanup --force >"$tmp/cleanup3.out" 2>&1
    check "cleanup --force does it" equals $? 0
}

# --- wall.sh status ----------------------------------------------------------------------------------------------

# The object NAME of the fake bucket.
sobj() { printf '%s/%s/%s\n' "$FAKE_S3" "$BUCKET" "$1"; }

# A run in the fake bucket, wall-NAME: a config (a window of 5, the chain to 100, the date 2026-10-22 00:00 UTC, and the
# settings NAME=VALUE after the name) and a ledger (the run began 2026-10-18 19:40:49 UTC, counted to 22:14:07 UTC).
fake_run() {
    local name="$1"
    shift
    mkdir -p "$(sobj "wall-$name/run")" "$(sobj "wall-$name/logs")" "$(sobj "wall-$name/ckpt")" \
        "$(sobj "wall-$name/results")"
    printf '%s\n' COMMIT=0123456789abcdef0123456789abcdef01234567 INSTANCE_TYPE=c8g.16xlarge CHAIN_TO=100 WINDOW=5 \
        GAME_HOURS=24 BUDGET_HOURS=36 PRICE=0.676 RUN_UNTIL=1792627200 ${@+"$@"} >"$(sobj "wall-$name/run/config")"
    printf 'start=1792352449\nboot=b\nbase=0\nhours=2.7711\nupdated=1792361647\n' >"$(sobj "wall-$name/run/ledger")"
}

# The solver's line for a game it recorded (solver.rs, summary_line): recorded_line N SCORE METHOD NODES SECONDS OPTIMAL
# PROOF [MORE], where OPTIMAL is the word after `optimal` (ok or unknown) and MORE is any text after the proof.
recorded_line() {
    printf 'n=%-5d score=%-9d %-7s nodes=%-11d %9.3f s  optimal %s  proof=%s%s\n' "$1" "$2" "$3" "$4" "$5" "$6" "$7" \
        "${8:-}"
}

# The solver's line for each of the games 1..LAST that it recorded, in logs/chain.out of the run NAME (NAME LAST).
fake_recorded() {
    local n
    for ((n = 1; n <= $2; n++)); do
        recorded_line "$n" $((n * 10)) search 0 1 ok exhausted
    done >"$(sobj "wall-$1/logs/chain.out")"
}

# The games of the table in the output $1 (normalized), in the order printed, as one line: "35 36 ... ".
table_games() {
    sed -n 's/^ \([0-9][0-9]*\) .*/\1/p' "$1" | tr '\n' ' '
}

# A progress line of the solver (engine.rs, progress_line): the game, the time since its search began, the nodes, the
# rate or how the search ended, and the best sum of promotions.
progress() {
    printf 'n=%s %s nodes %s (%s) best %s queue 3 spilled 0 | eff 99.9%% idle 0.0%% lock 0.0%% splits 1 steals 0 %s\n' \
        "$1" "$2" "$3" "$4" "$5" 'full 0.3ms ramp 0.0% tail 0.0% rebuild 0.0% first 0.0%'
}

# Sets the modification time of the file $1 to the epoch time $2 (touch -t reads UTC here).
stamp() {
    TZ=UTC touch -t "$(date -u -d "@$2" +%Y%m%d%H%M.%S 2>/dev/null || date -u -r "$2" +%Y%m%d%H%M.%S)" "$1"
}

# A time in seconds since the epoch in the zone TZ, as status prints it.
local_text() {
    date -d "@$1" '+%Y-%m-%d %H:%M %Z' 2>/dev/null || date -r "$1" '+%Y-%m-%d %H:%M %Z'
}

scenario_status() {
    echo "== wall.sh status: the run in progress, the games in flight, the times"
    local wall="$cloud/wall.sh" now saved f
    export BUCKET=taxman-013764204840-us-east-1
    rm -rf "${FAKE_S3:?}/$BUCKET"
    rm -rf "$FAKE_STATE"
    now="$(date +%s)"

    # A run that ended, and a prefix that is not a run: neither is in progress.  Its logs were saved on 2026-01-15, at
    # 03:30:00 UTC (19:30 the evening before at Pacific time, standard time).
    fake_run t-ended CHAIN_TO=40
    fake_recorded t-ended 40
    printf 'time=1768447800\nreason=complete: the range 1-40\n' >"$(sobj wall-t-ended/run/ended)"
    printf 'x\n' >"$(sobj wall-t-ended/results/strict.jsonl)"
    for f in results/strict.jsonl logs/chain.out; do stamp "$(sobj "wall-t-ended/$f")" 1768447800; done
    mkdir -p "$(sobj chain-9/run)"
    printf "x\n" >"$(sobj chain-9/run/config)"

    # The run in progress: it has recorded 1..40, and its window of 5 holds 41..45.  The games 35 to 40 are of every
    # kind; a replacement machine printed 37 and 38 again after an interruption (the first lines of 37 and 38 are the
    # earlier machine's); the search of 41 was stopped, and its line is incomplete.  The saved progress lines (the
    # oldest game, 41, runs; 42 is suspended; 43's search is done; 44 has no line; 45's only line is from before three
    # of 41's) and the clock of 41, which is the oldest game with its search running.
    fake_run t-live
    fake_recorded t-live 34
    {
        recorded_line 35 350 search 1133462769 4636.026 unknown exhausted
        recorded_line 36 360 repeat 0 0.240 unknown "same_search of 34"
        recorded_line 37 370 search 7 123.456 unknown exhausted
        recorded_line 38 380 search 9 1.5 unknown exhausted
        recorded_line 37 370 prime 0 0.055 unknown prime_formula
        recorded_line 38 380 known 0 0.003 ok "known from the committed chain (strict.jsonl)"
        recorded_line 39 7466640 search 0 0.247 unknown cap "  assumes statement U"
        recorded_line 40 400 search 248002917 812.5 unknown exhausted
        printf 'n=41    score=410       search  nodes=1000000000  6000.000 s  incomplete\n'
    } >>"$(sobj wall-t-live/logs/chain.out)"
    {
        progress 40 9m00s 100 done 1
        echo 'note: a line of the solver that is not a progress line'
        progress 45 2m00s 63 240.0k/s 500
        progress 41 1h38m00s 900000000 250.0k/s 341541
        progress 42 1h08m00s 63 0/s 341541
        progress 43 30m00s 4000 230.0k/s 777
        progress 41 1h39m00s 925000000 250.0k/s 341541
        progress 42 1h09m00s 63 0/s 341541
        progress 43 30m30s 5000 done 777
        progress 41 1h40m00s 1000000000 250.0k/s 341541
        progress 42 1h10m00s 63 0/s 341541
    } >"$(sobj wall-t-live/logs/chain.err)"
    printf 'x\n' >"$(sobj wall-t-live/results/strict.jsonl)"
    printf 'x\n' >"$(sobj wall-t-live/ckpt/n0041.ckpt)"
    printf '4500.036\n' >"$(sobj wall-t-live/ckpt/n0041.clock)"
    printf 'x\n' >"$(sobj wall-t-live/ckpt/n0042.ckpt)"
    # The last save: the checkpoints, 261 seconds ago.  The ledger (written every minute) is newer and not a save.
    for f in "$(sobj wall-t-live/results/strict.jsonl)" "$(sobj wall-t-live)"/logs/*; do stamp "$f" $((now - 900)); done
    stamp "$(sobj wall-t-live/logs/chain.err)" $((now - 290))
    for f in "$(sobj wall-t-live)"/ckpt/*; do stamp "$f" $((now - 261)); done
    saved="$(local_text $((now - 261)))"

    TZ=America/Los_Angeles env -u RUN "$wall" status >"$tmp/s1.out" 2>&1
    check "status with no RUN succeeds" equals $? 0
    sed 's/  */ /g' "$tmp/s1.out" >"$tmp/s1.norm"
    check "it takes the one run in progress" matches "$tmp/s1.norm" '^run t-live (s3://'
    check "and says that RUN was not set" matches "$tmp/s1.norm" 'the run in progress (RUN was not set)'
    check "it does not print the aws commands" lacks "$tmp/s1.out" '^+ aws'
    check "without a group, it says so" has_line "$tmp/s1.out" '  no group taxman-wall'
    check "it recorded through 40" has_line "$tmp/s1.norm" ' recorded through 40 (score 400)'
    check "the last save is the newest object saved, in Pacific time, and not the ledger's" matches "$tmp/s1.norm" \
        "^ last save to S3: $saved, 4 min [0-9]* s ago$"
    check "the oldest game runs, with the clock of the wall" has_line "$tmp/s1.norm" \
        ' 41 1h15m00s 1h40m00s 1,000,000,000 341,541 running, 250.0k nodes/s'
    check "a game whose threads are another's is suspended" has_line "$tmp/s1.norm" \
        ' 42 - 1h10m00s 63 341,541 suspended'
    check "a game whose search is done waits for the game before" has_line "$tmp/s1.norm" \
        ' 43 - 30m30s 5,000 777 search done, waiting for the game before'
    check "a game with no line has no state to tell" has_line "$tmp/s1.norm" ' 44 - - - - no recent progress line'
    check "and a game whose last line three of another's follow" has_line "$tmp/s1.norm" \
        ' 45 - - - - no recent progress line'
    check "the table has the last 6 games recorded, then the window after them, in order" equals \
        "$(table_games "$tmp/s1.norm")" '35 36 37 38 39 40 41 42 43 44 45 '
    check "its heading says what it holds" has_line "$tmp/s1.norm" \
        ' the last 6 games recorded and the 5 in flight (window 5; "own search" counts toward the wall of 24 hours):'
    check "one header line of columns stands over both" equals \
        "$(grep -c '^ n own search elapsed nodes best state$' "$tmp/s1.norm")" 1
    check "a searched game has its nodes and seconds, elapsed (1h17m16s), no wall clock and no best" has_line \
        "$tmp/s1.norm" ' 35 - 1h17m16s 1,133,462,769 - recorded, score 350 (search, exhausted)'
    check "a repeat has the game whose search it repeats" has_line "$tmp/s1.norm" \
        ' 36 - 0.2s 0 - recorded, score 360 (repeat, same_search of 34)'
    check "a game printed again is the last of its lines: a prime game" has_line "$tmp/s1.norm" \
        ' 37 - 0.1s 0 - recorded, score 370 (prime, prime_formula)'
    check "and a known one, with the words of its proof, and optimal ok if the solver had it in its table" has_line \
        "$tmp/s1.norm" \
        ' 38 - 0.0s 0 - recorded, score 380 (known, known from the committed chain (strict.jsonl), optimal ok)'
    check "the proof ends at what the solver adds to it" has_line "$tmp/s1.norm" \
        ' 39 - 0.2s 0 - recorded, score 7,466,640 (search, cap)'
    check "a recorded game's seconds are formatted as an in-flight game's" has_line "$tmp/s1.norm" \
        ' 40 - 13m32s 248,002,917 - recorded, score 400 (search, exhausted)'
    check "an incomplete line is not a recorded game" lacks "$tmp/s1.norm" 'score 410'
    check "every time is in Pacific time: the date (a day and a half ahead)" matches "$tmp/s1.norm" \
        'until 2026-10-21 17:00 PDT$'
    check "the run's beginning" matches "$tmp/s1.norm" 'the run began 2026-10-18 12:40 PDT, '
    check "and what the ledger counted to" matches "$tmp/s1.norm" 'counted to 2026-10-18 15:14 PDT$'
    check "none is in UTC" lacks "$tmp/s1.out" 'UTC\|[0-9]Z'

    TZ=Asia/Kolkata RUN=t-live "$wall" status >"$tmp/s2.out" 2>&1
    check "in India the date is the 22nd, in IST" matches "$tmp/s2.out" 'until 2026-10-22 05:30 IST$'
    check "and the run began in the small hours of the 19th" matches "$tmp/s2.out" 'the run began 2026-10-19 01:10 IST, '
    TZ=UTC RUN=t-live "$wall" status >"$tmp/s3.out" 2>&1
    check "in UTC it says UTC" matches "$tmp/s3.out" 'until 2026-10-22 00:00 UTC$'

    TZ=America/Los_Angeles RUN=t-live "$wall" status --verbose >"$tmp/s4.out" 2>&1
    check "--verbose prints the commands that read" matches "$tmp/s4.out" '^+ aws sts get-caller-identity'
    check "all of them" matches "$tmp/s4.out" '^+ aws s3 ls s3://.*/wall-t-live/'
    check "among them the read of the group" matches "$tmp/s4.out" '^+ aws autoscaling describe-auto-scaling-groups'
    TZ=America/Los_Angeles RUN=t-live VERBOSE=1 "$wall" status >"$tmp/s5.out" 2>&1
    check "so does VERBOSE=1" matches "$tmp/s5.out" '^+ aws sts get-caller-identity'

    # The group, with one instance, with two, with none, and when it cannot be read.
    printf 'x\n' >"$FAKE_STATE/asg"
    echo 1 >"$FAKE_STATE/desired"
    printf 'i-039bffbd48f5ab778\tus-east-1b\tInService\n' >"$FAKE_STATE/group-instances"
    TZ=America/Los_Angeles RUN=t-live "$wall" status >"$tmp/g1.out" 2>&1
    check "a group with an instance says its capacity" has_line "$tmp/g1.out" '  desired capacity 1'
    check "and the instance, its zone and its state" has_line "$tmp/g1.out" \
        '  instance i-039bffbd48f5ab778 in us-east-1b, InService'
    check "and no number is bare" lacks "$tmp/g1.out" '^  [0-9]'
    printf 'i-0123456789abcdef0\tus-east-1c\tPending\n' >>"$FAKE_STATE/group-instances"
    TZ=America/Los_Angeles RUN=t-live "$wall" status >"$tmp/g2.out" 2>&1
    check "a line for each of two instances" equals "$(grep -c '^  instance i-' "$tmp/g2.out")" 2
    check "the second with its own zone and state" has_line "$tmp/g2.out" \
        '  instance i-0123456789abcdef0 in us-east-1c, Pending'
    echo 0 >"$FAKE_STATE/desired"
    : >"$FAKE_STATE/group-instances"
    TZ=America/Los_Angeles RUN=t-live "$wall" status >"$tmp/g3.out" 2>&1
    check "a group at capacity 0" has_line "$tmp/g3.out" '  desired capacity 0'
    check "has no instance" has_line "$tmp/g3.out" '  no instance'
    FAKE_FAIL=describe-auto-scaling-groups TZ=America/Los_Angeles RUN=t-live "$wall" status >"$tmp/g4.out" 2>&1
    check "a group that cannot be read does not fail the status" equals $? 0
    check "which says so" has_line "$tmp/g4.out" '  cannot read the group taxman-wall'
    check "without the error text of aws" lacks "$tmp/g4.out" 'AccessDenied'
    check "and goes on with the chain" matches "$tmp/g4.out" '^  recorded through 40 '
    rm -f "$FAKE_STATE/asg" "$FAKE_STATE/desired" "$FAKE_STATE/group-instances"

    # The oldest game's search is done (it is being recorded), and a stopped search.
    {
        progress 41 1h41m00s 1001000000 done 341541
        progress 44 5m00s 70 stopped 9
    } >>"$(sobj wall-t-live/logs/chain.err)"
    TZ=America/Los_Angeles RUN=t-live "$wall" status >"$tmp/s6.out" 2>&1
    sed 's/  */ /g' "$tmp/s6.out" >"$tmp/s6.norm"
    check "the oldest game, searched, is being recorded" has_line "$tmp/s6.norm" \
        ' 41 1h15m00s 1h41m00s 1,001,000,000 341,541 search done, being recorded'
    check "a suspended game is still current after the oldest game's last line" has_line "$tmp/s6.norm" \
        ' 42 - 1h10m00s 63 341,541 suspended'
    check "a stopped search says so" has_line "$tmp/s6.norm" \
        ' 44 - 5m00s 70 9 search stopped, its checkpoint kept'

    # --recorded N: how many of the games recorded the table shows, before the command or after it.
    TZ=America/Los_Angeles RUN=t-live "$wall" status --recorded 2 >"$tmp/r1.out" 2>&1
    check "--recorded 2 succeeds" equals $? 0
    sed 's/  */ /g' "$tmp/r1.out" >"$tmp/r1.norm"
    check "it shows the last 2 games recorded, then those in flight" equals "$(table_games "$tmp/r1.norm")" \
        '39 40 41 42 43 44 45 '
    check "the heading counts them" has_line "$tmp/r1.norm" \
        ' the last 2 games recorded and the 5 in flight (window 5; "own search" counts toward the wall of 24 hours):'
    check "the line of the last game recorded stays" has_line "$tmp/r1.norm" ' recorded through 40 (score 400)'
    RUN=t-live "$wall" --recorded 2 status >"$tmp/r2.out" 2>&1
    sed 's/  */ /g' "$tmp/r2.out" >"$tmp/r2.norm"
    check "the option may come before the command" equals "$(table_games "$tmp/r2.norm")" '39 40 41 42 43 44 45 '
    RUN=t-live "$wall" status --recorded 0 >"$tmp/r3.out" 2>&1
    check "--recorded 0 succeeds" equals $? 0
    sed 's/  */ /g' "$tmp/r3.out" >"$tmp/r3.norm"
    check "it leaves the recorded games out" equals "$(table_games "$tmp/r3.norm")" '41 42 43 44 45 '
    check "and the heading is the games in flight's" has_line "$tmp/r3.out" \
        '  games in flight (window 5; "own search" counts toward the wall of 24 hours):'
    check "no row says recorded" lacks "$tmp/r3.out" 'recorded, score'
    RUN=t-live "$wall" status --recorded 100 >"$tmp/r4.out" 2>&1
    sed 's/  */ /g' "$tmp/r4.out" >"$tmp/r4.norm"
    check "more than the games recorded shows the 40 there are" equals "$(table_games "$tmp/r4.norm")" \
        "$(seq 1 45 | tr '\n' ' ')"
    check "and the heading counts the 40" has_line "$tmp/r4.norm" \
        ' the last 40 games recorded and the 5 in flight (window 5; "own search" counts toward the wall of 24 hours):'
    for value in x 2.5 -1 ''; do
        RUN=t-live "$wall" status --recorded "$value" >"$tmp/r5.out" 2>&1
        check "--recorded \"$value\" is refused" equals $? 1
        check "with the words for it" matches "$tmp/r5.out" "error: --recorded: \"$value\" is not a whole number"
    done
    check "and nothing was printed of the run" lacks "$tmp/r5.out" '^run t-live'
    RUN=t-live "$wall" status --recorded >"$tmp/r6.out" 2>&1
    check "--recorded with no value is refused" equals $? 1
    check "which says it needs one" matches "$tmp/r6.out" 'error: --recorded needs a value'
    "$wall" help >"$tmp/r7.out" 2>&1
    check "the usage text names the option with the command" matches "$tmp/r7.out" 'wall.sh status \[--recorded N\]'
    check "and gives its default" matches "$tmp/r7.out" '^  --recorded N .*default 6$'

    # A run that ended with the range recorded: no game in flight, the end in Pacific time (standard), the save a long
    # time ago.
    TZ=America/Los_Angeles RUN=t-ended "$wall" status >"$tmp/s7.out" 2>&1
    sed 's/  */ /g' "$tmp/s7.out" >"$tmp/s7.norm"
    check "an ended run shows its end in Pacific standard time" matches "$tmp/s7.norm" \
        '^ ENDED 2026-01-14 19:30 PST: complete: the range 1-40$'
    check "and its last save, long ago" matches "$tmp/s7.norm" \
        '^ last save to S3: 2026-01-14 19:30 PST, [0-9]* d [0-9]* h ago$'
    check "and no game in flight" matches "$tmp/s7.norm" 'no game in flight: the range to 40 is recorded'
    check "above it the last 6 games recorded" equals "$(table_games "$tmp/s7.norm")" '35 36 37 38 39 40 '
    check "under a heading that holds no game in flight" has_line "$tmp/s7.norm" ' the last 6 games recorded:'
    check "and the note comes after them" equals "$(tail -n 1 "$tmp/s7.norm")" \
        ' no game in flight: the range to 40 is recorded'
    TZ=America/Los_Angeles RUN=t-ended "$wall" status --recorded 0 >"$tmp/s7b.out" 2>&1
    check "with --recorded 0 an ended run's status succeeds" equals $? 0
    check "and has the note" matches "$tmp/s7b.out" '^  no game in flight: the range to 40 is recorded$'
    check "without a table" lacks "$tmp/s7b.out" 'own search'
    check "or a heading" lacks "$tmp/s7b.out" '^  the last .* recorded'
    check "or a recorded game" lacks "$tmp/s7b.out" 'recorded, score'

    # Two runs in progress: status lists them and asks for RUN.
    fake_run t-live2
    env -u RUN "$wall" status >"$tmp/s8.out" 2>&1
    check "two runs in progress is an error" equals $? 1
    check "which says so" matches "$tmp/s8.out" 'more than one run is in progress'
    check "and lists them" equals "$(grep -c '^    t-live2\{0,1\}$' "$tmp/s8.out")" 2
    check "the ended run is not one of them" lacks "$tmp/s8.out" 't-ended'
    # None: only the ended run is left.
    rm -rf "$(sobj wall-t-live)" "$(sobj wall-t-live2)"
    env -u RUN "$wall" status >"$tmp/s9.out" 2>&1
    check "no run in progress is an error" equals $? 1
    check "which says so" matches "$tmp/s9.out" 'no run is in progress'

    # A run whose machine has not saved yet: logs/chain.err is not in S3, and the status is whole.
    fake_run t-early
    env -u RUN "$wall" status >"$tmp/s10.out" 2>&1
    check "a run before its first save is no error" equals $? 0
    check "it takes the run" matches "$tmp/s10.out" '^run t-early  (s3://'
    check "it says no progress line is saved" has_line "$tmp/s10.out" '  no progress lines saved yet'
    check "in place of the games in flight" lacks "$tmp/s10.out" 'no recent progress line'
    check "and nothing is saved at all" matches "$tmp/s10.out" '^  nothing has been saved to S3 yet$'
    rm -rf "$(sobj wall-t-early)"

    # No game recorded yet (only an incomplete line), and a progress line: the table has no recorded game, and its
    # heading claims none.
    fake_run t-new
    printf 'n=1     score=10        search  nodes=5          2.000 s  incomplete\n' >"$(sobj wall-t-new/logs/chain.out)"
    progress 1 1m00s 5000 120.0k/s 0 >"$(sobj wall-t-new/logs/chain.err)"
    RUN=t-new "$wall" status >"$tmp/n1.out" 2>&1
    check "a run with no game recorded succeeds" equals $? 0
    sed 's/  */ /g' "$tmp/n1.out" >"$tmp/n1.norm"
    check "it says no game is recorded" has_line "$tmp/n1.out" '  recorded through: no game yet'
    check "the table is the games in flight" equals "$(table_games "$tmp/n1.norm")" '1 2 3 4 5 '
    check "under their heading" has_line "$tmp/n1.out" \
        '  games in flight (window 5; "own search" counts toward the wall of 24 hours):'
    check "with a game that runs" has_line "$tmp/n1.norm" ' 1 - 1m00s 5,000 0 running, 120.0k nodes/s'
    check "and no game recorded" lacks "$tmp/n1.out" 'recorded, score\|^  the last'
    rm -rf "$(sobj wall-t-new)"

    # Fewer games recorded than the table shows: those there are.
    fake_run t-few
    fake_recorded t-few 3
    progress 4 1m00s 5000 120.0k/s 0 >"$(sobj wall-t-few/logs/chain.err)"
    RUN=t-few "$wall" status >"$tmp/f1.out" 2>&1
    sed 's/  */ /g' "$tmp/f1.out" >"$tmp/f1.norm"
    check "with 3 games recorded the table has 3" equals "$(table_games "$tmp/f1.norm")" '1 2 3 4 5 6 7 8 '
    check "and its heading says 3" has_line "$tmp/f1.norm" \
        ' the last 3 games recorded and the 5 in flight (window 5; "own search" counts toward the wall of 24 hours):'
    RUN=t-few "$wall" status --recorded 1 >"$tmp/f2.out" 2>&1
    sed 's/  */ /g' "$tmp/f2.out" >"$tmp/f2.norm"
    check "one game is not games" has_line "$tmp/f2.norm" \
        ' the last game recorded and the 5 in flight (window 5; "own search" counts toward the wall of 24 hours):'
    check "and it is the last recorded" equals "$(table_games "$tmp/f2.norm")" '3 4 5 6 7 8 '
    rm -rf "$(sobj wall-t-few)"

    # Games recorded and no progress lines saved: the recorded games, then the note for the games in flight.
    fake_run t-quiet
    fake_recorded t-quiet 3
    RUN=t-quiet "$wall" status >"$tmp/q1.out" 2>&1
    check "recorded games and no chain.err succeeds" equals $? 0
    sed 's/  */ /g' "$tmp/q1.out" >"$tmp/q1.norm"
    check "the table has the recorded games only" equals "$(table_games "$tmp/q1.norm")" '1 2 3 '
    check "under the heading of both" has_line "$tmp/q1.norm" \
        ' the last 3 games recorded and the 5 in flight (window 5; "own search" counts toward the wall of 24 hours):'
    check "and the note for the games in flight follows" equals "$(tail -n 1 "$tmp/q1.out")" \
        '  no progress lines saved yet'
    check "in place of their rows" lacks "$tmp/q1.out" 'no recent progress line'
    rm -rf "$(sobj wall-t-quiet)"

    check_status_host
    default_run
}

# --- wall.sh status --host ---------------------------------------------------------------------------------------

# A chain on another machine, the home directory $FAKE_HOME of the stand-in for ssh: the results file is recorded
# through 3 (its last line is a game not completed), logs/chain.out has the game 2 only (the games of the machine's own
# run), the window is 3 and game 4 has a clock.
check_status_host() {
    echo "== wall.sh status --host: a chain on another machine, over ssh"
    local wall="$cloud/wall.sh" work="$tmp/home/taxman-work" calls
    export FAKE_HOME="$tmp/home"
    mkdir -p "$work/results" "$work/logs" "$work/ckpt"
    printf '%s\n' '{"schema":5,"n":2,"score":20,"moves":[2],"completed":true}' \
        '{"schema":5,"n":3,"score":7466640,"moves":[3],"completed":true}' \
        '{"schema":5,"n":4,"score":40,"moves":[4],"completed":false}' >"$work/results/strict.jsonl"
    printf '%s chain 1-100; threads: all available; window 3; wall: 48 hours; results x; checkpoints y\n' \
        2026-10-21T00:00:00Z >"$work/logs/chain.log"
    recorded_line 2 20 search 5 1.5 unknown exhausted >"$work/logs/chain.out"
    progress 4 1h40m00s 14000000000 250.0k/s 341541 >"$work/logs/chain.err"
    printf '7200.5\n' >"$work/ckpt/n0004.clock"
    : >"$FAKE_LOG"
    TZ=America/Los_Angeles env -u RUN "$wall" status --host user@fake.example >"$tmp/h1.out" 2>&1
    calls="$?,$(wc -l <"$FAKE_LOG" | tr -d ' ')"
    sed 's/  */ /g' "$tmp/h1.out" >"$tmp/h1.norm"
    check "status --host succeeds with no RUN, no profile and no aws call" equals "$calls" 0,0
    check "it is recorded through the last game completed of the results file, not of chain.out" has_line \
        "$tmp/h1.norm" ' recorded through 3 (score 7,466,640)'
    check "the table has the game of chain.out, the game in flight with its clock, and the window of chain.log" equals \
        "$(grep -c -e ' 2 - 1.5s 5 - recorded, score 20 (search, exhausted)$' \
            -e ' 4 2h00m00s 1h40m00s 14,000,000,000 341,541 running, 250.0k nodes/s$' \
            -e ' 6 - - - - no recent progress line$' "$tmp/h1.norm")" 3
    check "a node count of 14,000,000,000 has its commas and ends under the header's right-aligned nodes" equals \
        "$(awk '/ own search / {head = index($0, "nodes") + 4}
            /14,000,000,000/ {row = index($0, "14,000,000,000") + 13}
            END {print (head > 0 && head == row) ? "ok" : "head " head " row " row}' "$tmp/h1.out")" ok
}

# --- wall.sh stop and cleanup: the run in progress ---------------------------------------------------------------

# stop and cleanup take the run in progress when RUN is not set, as status does, and launch takes run1.  The bucket holds
# the ended run t-ended, and no run is in progress.
default_run() {
    echo "== wall.sh stop and cleanup: the run in progress when RUN is not set"
    local wall="$cloud/wall.sh" commit=0123456789abcdef0123456789abcdef01234567 writes
    export WALL_DIR="$tmp/wall"
    # What the log of the fake aws holds of the calls that write: the stop and the end objects, the group, the rest.
    writes='^s3 cp [^ ]* s3://\|set-desired-capacity\|delete-\|create-\|put-'

    env -u RUN DRY_RUN=1 "$wall" launch --commit "$commit" --until 2099-01-01 >"$tmp/d0.out" 2>&1
    check "launch with no RUN takes run1, in progress or not" matches "$tmp/d0.out" 'run run1: s3://.*/wall-run1'

    : >"$FAKE_LOG"
    env -u RUN "$wall" stop >"$tmp/d1.out" 2>&1
    check "a stop with no run in progress is an error" equals $? 1
    check "which says so" matches "$tmp/d1.out" 'no run is in progress'
    env -u RUN "$wall" stop --now >"$tmp/d2.out" 2>&1
    check "so is stop --now" equals $? 1
    env -u RUN "$wall" cleanup --force >"$tmp/d3.out" 2>&1
    check "so is cleanup, --force or not" equals $? 1
    check "which says so too" matches "$tmp/d3.out" 'no run is in progress'
    check "none of them wrote anything, or touched the account" lacks "$FAKE_LOG" "$writes"
    check "nor took run1 for the run" lacks "$tmp/d1.out" 'run run1'

    # Two in progress: neither is chosen.
    fake_run t-live
    fake_run t-live2
    : >"$FAKE_LOG"
    env -u RUN "$wall" stop >"$tmp/d4.out" 2>&1
    check "a stop with two runs in progress is an error" equals $? 1
    check "which says so" matches "$tmp/d4.out" 'more than one run is in progress'
    check "and lists them" equals "$(grep -c '^    t-live2\{0,1\}$' "$tmp/d4.out")" 2
    env -u RUN "$wall" cleanup >"$tmp/d5.out" 2>&1
    check "so is a cleanup" equals $? 1
    check "which says so" matches "$tmp/d5.out" 'more than one run is in progress'
    check "neither run is stopped" absent "$(sobj wall-t-live/run/stop)"
    check "nor the other" absent "$(sobj wall-t-live2/run/stop)"
    check "nothing was written, or touched in the account" lacks "$FAKE_LOG" "$writes"
    RUN=t-live2 "$wall" stop >"$tmp/d6.out" 2>&1
    check "RUN names the run when there are several" equals $? 0
    check "and it is the one that is stopped" exists "$(sobj wall-t-live2/run/stop)"
    check "not the other" absent "$(sobj wall-t-live/run/stop)"
    check "the log does not call it the run in progress" lacks "$tmp/d6.out" 'the run in progress'
    rm -rf "$(sobj wall-t-live2)"

    # One in progress (t-live), and the run that ended beside it.
    env -u RUN "$wall" cleanup >"$tmp/d7.out" 2>&1
    check "a cleanup with no RUN takes the run in progress" equals $? 1
    check "which has not ended, and is named" matches "$tmp/d7.out" 'wall-t-live/run/ended is not there'
    env -u RUN "$wall" stop >"$tmp/d8.out" 2>&1
    check "a stop with one run in progress succeeds" equals $? 0
    check "it says which run, and why" matches "$tmp/d8.out" 'run t-live (the run in progress: RUN was not set)'
    check "it puts the stop object in that run" exists "$(sobj wall-t-live/run/stop)"
    check "and in the run that ended" absent "$(sobj wall-t-ended/run/stop)"
    env -u RUN "$wall" stop --now >"$tmp/d9.out" 2>&1
    check "stop --now ends that run" matches "$(sobj wall-t-live/run/ended)" 'stopped at once'
    check "and takes the group to 0" has_line "$FAKE_STATE/desired" 0

    # None again: the run has ended, and cleanup needs its name.
    env -u RUN "$wall" cleanup >"$tmp/d10.out" 2>&1
    check "a cleanup with no RUN finds no run in progress once the run has ended" matches "$tmp/d10.out" \
        'no run is in progress'
    RUN=t-live "$wall" cleanup >"$tmp/d11.out" 2>&1
    check "and RUN names the run to clean up" equals $? 0
    check "its end checks are printed" matches "$tmp/d11.out" '^+ aws ec2 describe-volumes'
}

# --- sweep.sh ----------------------------------------------------------------------------------------------------

# The lines of the file $1 for game $2 (completed or not), one a line.
game_lines() {
    grep -F -- "{\"schema\":5,\"n\":$2," "$1"
}

# Whether the sweep of the merged file $1 over $2..$3 has nothing left to search again (sweep_plan.py's count is 0).
converged() {
    equals "$(python3 -I "$TAXMAN_ROOT/solver/tools/sweep_plan.py" excerpt "$1" "$2" "$3" /dev/null | awk '{print $2}')" 0
}

scenario_sweep() {
    echo "== sweep.sh: blocks, the passes over the merged file, resuming"
    local w="$tmp/sweep" res known pid
    # The real referees (verify.sh checks the merged file with both, and against the committed chain).
    cp "$real_cloud/verify.sh" "$cloud/verify.sh"
    : >"$FAKE_LOG"

    # 1301..1340 in four blocks and one second a game: the first block chains from the known games (1..1300), the other
    # blocks start standalone; the passes then search again the games that have more to give.
    known="$tmp/known1300.jsonl"
    head -n 1300 "$TAXMAN_ROOT/results/strict.jsonl" >"$known"
    (
        export TAXMAN_WORK="$w" KNOWN="$known" BLOCKS=4 GAME_SECONDS=1 PROGRESS=0 MERGE_EVERY=5 S3_DEST=s3://b/sweep
        export SAVE_EVERY=3600
        "$cloud/sweep.sh" 1301 1340 >"$tmp/sweep.out" 2>&1
        echo $? >"$tmp/sweep.status"
    )
    res="$w/results/explore.jsonl"
    check "the sweep completes" has_line "$tmp/sweep.status" 0
    check "the outcome says so" matches "$w/sweep.outcome" '^complete: 1301\.\.1340 in 4 blocks'
    check "both referees accepted the merged file" matches "$tmp/sweep.out" 'both referees agree: no problem found'
    check "the plan is four blocks that tile 1301..1340" equals "$(awk '
        NR == 1 && $1 != 1301 {bad = 1} NR > 1 && $1 != last + 1 {bad = 1} {last = $2}
        END {print (NR == 4 && last == 1340 && !bad) ? "ok" : "bad"}' "$w/results/explore-blocks/plan")" ok
    check "the first block chains from the known games" matches "$res" '"n":1301,.*"prev":{"n":1300,'
    check "a block's first game has no game before it" matches "$res" '"n":1311,.*"prev":null'
    check "every game of the sweep says which pass made it" equals \
        "$(grep -E '^\{"schema":5,"n":(130[1-9]|13[1-3][0-9]|1340),' "$res" | grep -v -c '"pass":')" 0
    check "nothing is left that has more to give" converged "$res" 1301 1340
    check "the merged file is in S3" exists "$(obj sweep/results/explore.jsonl)"
    check "and is the one on the machine" cmp -s "$res" "$(obj sweep/results/explore.jsonl)"
    check "the blocks are in S3 too" exists "$(obj sweep/results/explore-blocks/block-004.jsonl)"

    # The same command again changes nothing.
    cp "$res" "$tmp/explore-before.jsonl"
    (
        export TAXMAN_WORK="$w" KNOWN="$known" BLOCKS=4 GAME_SECONDS=1 PROGRESS=0
        "$cloud/sweep.sh" 1301 1340 >"$tmp/sweep2.out" 2>&1
        echo $? >"$tmp/sweep2.status"
    )
    check "a rerun completes" has_line "$tmp/sweep2.status" 0
    check "and leaves the merged file as it was" cmp -s "$res" "$tmp/explore-before.jsonl"
    (
        export TAXMAN_WORK="$w" KNOWN="$known" GAME_SECONDS=1 PROGRESS=0
        "$cloud/sweep.sh" 1301 1341 >"$tmp/sweep3.out" 2>&1
    )
    check "another range over the same files is refused" matches "$tmp/sweep3.out" 'is the plan for 1301\.\.1340'

    # The boundary: block 2 begins at 1413, which standalone needs more than the limit and with the game before it, 1412,
    # proven (which a chain has) is settled at once by the ceiling.  Pass 2 searches it again with 1412, and the games
    # after it go on from it, as in an unbroken chain.
    # The known file here goes past FIRST - 1 (to 1420): the games of the range it holds are not taken.
    local w2="$tmp/sweep-boundary"
    head -n 1420 "$TAXMAN_ROOT/results/strict.jsonl" >"$tmp/known1420.jsonl"
    mkdir -p "$w2/results/explore-blocks"
    printf '1411 1412\n1413 1416\n' >"$w2/results/explore-blocks/plan"
    (
        export TAXMAN_WORK="$w2" KNOWN="$tmp/known1420.jsonl" GAME_SECONDS=0.3 PROGRESS=0 MERGE_EVERY=5
        "$cloud/sweep.sh" 1411 1416 >"$tmp/boundary.out" 2>&1
        echo $? >"$tmp/boundary.status"
    )
    res="$w2/results/explore.jsonl"
    check "the boundary sweep completes, both referees agreeing" has_line "$tmp/boundary.status" 0
    check "the known games past 1410 are not taken: every line of 1411..1416 is the sweep's" equals \
        "$(grep -E '^\{"schema":5,"n":141[1-6],' "$res" | grep -v -c '"pass":')" 0
    check "1413 was unfinished in pass 1, with no game before it" matches "$res" \
        '"n":1413,"score":null,.*"prev":null.*"pass":1'
    check "and completed in pass 2 by the ceiling of 1412" matches "$res" \
        '"n":1413,"score":[0-9]*,.*"proof":"cap".*"prev":{"n":1412,.*"pass":2'
    check "1414 repeats the search of 1413" matches "$res" '"n":1414,"score":[0-9]*,.*"proof":"same_search".*"pass":2'
    check "1416 needs a search longer than the limit, as in a chain" matches "$res" '"n":1416,"score":null,.*"pass":2'
    check "nothing is left that has more to give at 1411..1416" converged "$res" 1411 1416

    # A SIGTERM stops it, with a line in the outcome and the games done kept; the same command goes on to the end.
    local w3="$tmp/sweep-stop"
    (
        export TAXMAN_WORK="$w3" KNOWN="$known" BLOCKS=4 GAME_SECONDS=1 PROGRESS=0 MERGE_EVERY=2
        "$cloud/sweep.sh" 1301 1340 >"$tmp/stop1.out" 2>&1
        echo $? >"$tmp/stop1.status"
    ) &
    pid=$!
    wait_for matches "$w3/results/explore-blocks/block-004.jsonl" '"n":1331,'
    kill -TERM "$(cat "$w3/sweep.pid")"
    wait "$pid"
    check "a SIGTERM stops the sweep with status 143" has_line "$tmp/stop1.status" 143
    check "and the outcome says stopped" matches "$w3/sweep.outcome" '^stopped by a signal$'
    check "the games done are in the merged file" matches "$w3/results/explore.jsonl" '"n":1331,'
    (
        export TAXMAN_WORK="$w3" KNOWN="$known" GAME_SECONDS=1 PROGRESS=0
        "$cloud/sweep.sh" 1301 1340 >"$tmp/stop2.out" 2>&1
        echo $? >"$tmp/stop2.status"
    )
    check "the same command completes it, both referees agreeing" has_line "$tmp/stop2.status" 0
    check "with every game 1301..1340 either completed or unfinished, none missing" equals \
        "$(for n in $(seq 1301 1340); do game_lines "$w3/results/explore.jsonl" "$n" | head -n 1; done | wc -l | tr -d ' ')" 40
    check "and nothing left that has more to give" converged "$w3/results/explore.jsonl" 1301 1340
    cp "$tmp/verify.sh.stub" "$cloud/verify.sh"
}

for s in "${selected[@]}"; do
    case "$s" in
        chain) scenario_chain ;;
        ledger) scenario_ledger ;;
        node) scenario_node ;;
        launch) scenario_launch ;;
        status) scenario_status ;;
        sweep) scenario_sweep ;;
        *) echo "unknown scenario $s" >&2; exit 2 ;;
    esac
done
echo
if [ "$failures" = 0 ]; then echo "all checks passed"; else echo "$failures checks FAILED"; fi
exit "$failures"
