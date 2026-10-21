#!/usr/bin/env bash
# The unattended run to the wall: one spot instance in an Auto Scaling group that replaces it after an interruption,
# running the chain from n = 1 until a game's own search passes the wall, the budget of instance-hours is used, the date
# comes, or something fails.  This is the script for your machine; the instance's side is wall-node.sh
# (solver/docs/runbook/wall.md).
#
#   solver/tools/cloud/wall.sh launch --commit SHA --until DATE [options]   create everything and start the run
#   solver/tools/cloud/wall.sh status [--recorded N]                         where the run is, from S3 and the group
#   solver/tools/cloud/wall.sh status [--recorded N] --host USER@HOST        where the chain is, over ssh
#   solver/tools/cloud/wall.sh stop [--now]                                  end the run (cleanly, or at once)
#   solver/tools/cloud/wall.sh cleanup [--force]                             delete what launch made; check what is left
#
# Every aws command that creates, changes or deletes something is printed, as `+ aws ...`, before it runs; DRY_RUN=1
# prints those without running them.  The commands that only read are printed too with --verbose (any command takes it;
# environment variable VERBOSE=1).  `launch` does, in this order:
#
#   1. writes $S3_DEST/run/config, the run's settings (the commit, the budget, the date, the wall, ...), which every
#      machine reads
#   2. puts the inline policy `taxman-wall` on the role taxman-ec2 (wall-policy.json): a machine may set the group's
#      desired capacity (to 0, at the end of the run) and read the group
#   3. creates a security group `taxman-wall` with no inbound rule, and 4. the launch template `taxman-wall`, whose user
#      data installs, checks out the commit, and starts wall-node.sh in tmux
#   5. creates the Auto Scaling group `taxman-wall` (one spot instance of the type, the cheapest zone first, capacity
#      rebalancing off) at desired capacity 0, 6. puts a scheduled action that takes it to 0 at the date plus two hours,
#      whatever the machines do, and 7. sets desired capacity 1: the first machine starts
#
# launch options (each also an environment variable):
#   --commit SHA          COMMIT          the commit the machines check out (40 hex digits; pushed to origin)
#   --until DATE          UNTIL           the hard date (UTC): YYYY-MM-DD or YYYY-MM-DDTHH:MM:SSZ
#   --budget-hours H      BUDGET_HOURS    instance-hours in all, over every machine                   default 74
#   --game-hours H        GAME_HOURS      the wall: one game's own search, in hours                   default 24
#   --chain-to N          CHAIN_TO        the last game                                              default 8191
#   --window K            WINDOW          games in flight                                             default 4
#   --threads K           THREADS         worker threads                                              default all
#   --instance-type T     INSTANCE_TYPE   the type of the one instance                              default c8g.16xlarge
#   --zones "Z1 Z2"       ZONES           availability zones, cheapest first                          default every zone
#                                         that offers the type, by the spot price of the last day
#   --spot-max P          SPOT_MAX        the most an hour of spot may cost, dollars                  default 1.0
#   --price P             PRICE           what an instance-hour is counted at in `status`, dollars    default 0.676
#   --volume-gb G         VOLUME_GB       the root volume                                             default 40
#   --extra-args "..."    EXTRA_ARGS      more flags for `taxman solve`; "--use-prev" makes the chain draw on the known games
#                                         (the committed chain, the proven boards of the bounds store, ...) of the
#                                         pinned commit's results/ (solver/docs/runbook/wall.md)
#   --margin S            LEDGER_MARGIN   seconds before the budget or the date that the chain stops  default 300
#   --max-instances N     MAX_INSTANCES   machines in all before the run ends as a boot loop         default 20
#   --save-every S        SAVE_EVERY      seconds between the copies to S3 (run.sh)                  default 600
#   --checkpoint-interval S               seconds between checkpoints (CHECKPOINT_INTERVAL, chain.sh) default 300
#   --continue                            go on with a run that has ended or been stopped (its S3 state is kept; the
#                                         ended and stop objects are removed, the config is written again)
# The build cache (solver/docs/runbook/build-cache.md), by environment variables only; the config carries them to each
# machine's setup.sh: RELEASE_TESTS=1 (a machine whose cache has no tested build of the commit builds it, runs the
# release suite and, if it passes, stores the binaries; if it fails the run ends with RELEASE_TESTS_FAILED in $S3_DEST),
# REQUIRE_CACHED=1 (a machine whose cache has none ends the run instead of building).
# status reads the run's state in S3 and the group: the run, the ledger, the group, and from the last save to S3 (its
# time, in the local time zone (TZ), and how long ago) the last game recorded and a table, a line for each of the last
# games recorded and then for each game in flight (solver/docs/runbook/wall.md, "Launch, watch, stop, clean up").  Its
# options go before the command or after it, as --verbose does:
#   --recorded N          the games recorded that the table shows: the last N (0: none)               default 6
#   --host USER@HOST      read the state of a chain that runs on that machine (chain.sh in the tmux session tx, its
#                         state in ~/taxman-work) in one ssh call, instead of S3 and the group: no AWS profile, no aws
#                         call.  It shows the machine's uptime, whether the chain runs, how it ended (chain.outcome),
#                         the window and the wall (chain.log), when logs/chain.err last changed, the game the results
#                         file is recorded through, and the same table.  ssh runs with BatchMode: the key must need no
#                         password
# Other environment: RUN (the name of the run; its state is $S3_DEST = s3://BUCKET/wall-RUN; default run1 for launch,
# and for status, stop and cleanup the one run in progress, the one whose prefix has a config and no ended object; with
# none they refuse and say so, and with several they refuse and list the runs to choose from),
# BUCKET, AWS_PROFILE (never defaulted: export it, aws login --profile NAME) and AWS_DEFAULT_REGION (us-east-1),
# DRY_RUN, VERBOSE, AWS_CLI, SSH_CLI (ssh), and WALL_DIR (default ~/taxman-wall/RUN): where the files that the commands
# take are written and kept (the user data, the launch template's data, the config).  The group, template, policy and
# security group are always named taxman-wall: there is one run at a time.

set -u
set -o pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=env.sh
. "$here/env.sh"

AWS_CLI="${AWS_CLI:-aws}"
SSH_CLI="${SSH_CLI:-ssh}"
export AWS_DEFAULT_REGION="${AWS_DEFAULT_REGION:-us-east-1}"
BUCKET="${BUCKET:-taxman-013764204840-us-east-1}"
NAME=taxman-wall
ROLE=taxman-ec2
DRY_RUN="${DRY_RUN:-0}"
VERBOSE="${VERBOSE:-0}"
SHOW_RECORDED=6
WALL_HOST=""
TAGS='{Key=Project,Value=taxman},{Key=Purpose,Value=wall}'
IMAGE="resolve:ssm:/aws/service/ami-amazon-linux-latest/al2023-ami-kernel-default-arm64"

# The run: RUN names it, S3_DEST is its state, and `work` is where the files that the commands take (the user data, the
# template, the config, ...) are written and kept, so that a command that was printed can be run again by hand.
set_run() {
    RUN="$1"
    S3_DEST="s3://$BUCKET/wall-$RUN"
    work="${WALL_DIR:-$HOME/taxman-wall/$RUN}"
}
RUN="${RUN:-}"
RUN_PICKED=0
if [ -n "$RUN" ]; then set_run "$RUN"; fi

# --- aws, printed ------------------------------------------------------------------------------------------------

# Prints a command as the shell would take it back: an argument with anything but plain characters is in single quotes.
show() {
    local arg out="+ aws"
    for arg in "$@"; do
        case "$arg" in
            '' | *[!A-Za-z0-9_./:=@%+,-]*) out="$out '$(printf '%s' "$arg" | sed "s/'/'\\\\''/g")'" ;;
            *) out="$out $arg" ;;
        esac
    done
    printf '%s\n' "$out" >&2
}

# Prints a command that only reads, with --verbose.
show_read() {
    if [ "$VERBOSE" = 1 ]; then show "$@"; fi
}

# A command that only reads.
read_aws() {
    show_read "$@"
    "$AWS_CLI" "$@"
}

# A command that only reads, and whose error text is left out: the caller says what failed.
read_aws_quiet() {
    show_read "$@"
    "$AWS_CLI" "$@" 2>/dev/null
}

# A command that only reads, whose output is a check that is meaningless without the command: always printed.
check_aws() {
    show "$@"
    "$AWS_CLI" "$@"
}

# A command that creates, changes or deletes something: not run when DRY_RUN=1.
change_aws() {
    show "$@"
    if [ "$DRY_RUN" = 1 ]; then
        echo "  (dry run: not run)" >&2
        return 0
    fi
    "$AWS_CLI" "$@"
}

# change_aws for a command whose output is wanted: the first argument is what a dry run answers in its place.
change_aws_value() {
    local placeholder="$1"
    shift
    if [ "$DRY_RUN" = 1 ]; then
        show "$@"
        echo "  (dry run: not run)" >&2
        echo "$placeholder"
        return 0
    fi
    change_aws "$@"
}

# The text of the object PATH of S3_DEST (run/config, logs/chain.err, ...); the status of aws, and no text, if it is not
# there.
s3_text() {
    read_aws_quiet s3 cp "$S3_DEST/$1" - --only-show-errors
}

# `aws s3 ls`, its times in UTC (it prints them in the time zone of the machine otherwise).
s3_ls() {
    TZ=UTC read_aws_quiet s3 ls "$@"
}

# Whether the object NAME is in S3_DEST/run.
has_object() {
    local listing
    listing="$(s3_ls "$S3_DEST/run/$1")"
    printf '%s\n' "$listing" | grep -q " $1\$"
}

# The value of NAME in the text on stdin (a config or a ledger).
field() {
    sed -n "s/^$1=//p" | tail -n 1
}

# --- helpers -----------------------------------------------------------------------------------------------------

# The seconds since the epoch of a UTC date, YYYY-MM-DD or YYYY-MM-DDTHH:MM:SSZ (GNU date, then BSD date).
to_epoch() {
    local text="$1"
    case "$text" in
        ????-??-??) text="${text}T00:00:00Z" ;;
        ????-??-??T??:??:??Z) ;;
        *) return 1 ;;
    esac
    date -u -d "$text" +%s 2>/dev/null || date -u -j -f '%Y-%m-%dT%H:%M:%SZ' "$text" +%s 2>/dev/null
}

# The comment at the top of this file.
usage() {
    sed -n '2,/^$/p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
}

is_number() {
    case "$1" in '' | *[!0-9.]* | *.*.*) return 1 ;; esac
    return 0
}

is_whole_number() {
    case "$1" in '' | *[!0-9]*) return 1 ;; esac
    return 0
}

# A span of SECONDS in words: 45 s, 4 min 30 s, 2 h 5 min, 3 d 4 h.
span_text() {
    awk -v s="$1" 'BEGIN {
        if (s < 0) s = 0
        if (s < 60) printf "%d s\n", s
        else if (s < 3600) printf "%d min %d s\n", s / 60, s % 60
        else if (s < 86400) printf "%d h %d min\n", s / 3600, s / 60 % 60
        else printf "%d d %d h\n", s / 86400, s / 3600 % 24 }'
}

# Sets ACCOUNT to the account of the credentials, which must be the one the bucket carries.
check_account() {
    ACCOUNT="$(read_aws sts get-caller-identity --query Account --output text)" ||
        die "no AWS credentials (aws login --profile bvc, then export AWS_PROFILE=bvc)"
    case "$BUCKET" in
        *"$ACCOUNT"*) ;;
        *) die "the bucket $BUCKET does not carry the account number $ACCOUNT: wrong profile?" ;;
    esac
}

# Checks the account, and takes the run in progress as the run when RUN is not set (pick_run); status, stop and cleanup
# go by this, and launch, which takes run1 when RUN is not set, never needs the pick.
need_run() {
    check_account
    [ -n "$RUN" ] || pick_run
}

need_account() {
    need_run
    local picked=""
    if [ "$RUN_PICKED" = 1 ]; then picked=" (the run in progress: RUN was not set)"; fi
    log "account $ACCOUNT, region $AWS_DEFAULT_REGION, run $RUN$picked: $S3_DEST"
}

# Takes the run in progress as the run when RUN is not set: the one run whose prefix in the bucket (wall-NAME) has a
# config and no ended object.  If several are in progress, or none, it says so and asks for RUN=NAME.
pick_run() {
    local name found=""
    for name in $(s3_ls "s3://$BUCKET/wall-" | sed -n 's|^ *PRE wall-\(.*\)/$|\1|p'); do
        set_run "$name"
        if has_object config && ! has_object ended; then found="$found $name"; fi
    done
    case "$found" in
        '') die "no run is in progress in s3://$BUCKET (no prefix wall-NAME has a run/config and no run/ended):" \
            "RUN=NAME names the run, one that has ended included" ;;
        ' '*' '*) die "more than one run is in progress; RUN=NAME says which:$(printf '\n    %s' $found)" ;;
    esac
    set_run "${found# }"
    RUN_PICKED=1
}

# --- launch ------------------------------------------------------------------------------------------------------

# The zones in which the type is offered, the cheapest spot price of the last day first.
zones_by_price() {
    local type="$1" offered prices zone
    offered="$(read_aws ec2 describe-instance-type-offerings --location-type availability-zone \
        --filters "Name=instance-type,Values=$type" --query 'InstanceTypeOfferings[].Location' --output text)" ||
        die "cannot read where $type is offered"
    prices="$(read_aws ec2 describe-spot-price-history --instance-types "$type" --product-descriptions Linux/UNIX \
        --start-time "$(epoch_date $(($(date +%s) - 86400)))" \
        --query 'SpotPriceHistory[].[Timestamp,AvailabilityZone,SpotPrice]' --output text)" ||
        die "cannot read the spot price history of $type"
    # The latest price of each zone (the history is newest first), then the zones by price; a zone with none, last.
    for zone in $offered; do
        printf '%s %s\n' "$(printf '%s\n' "$prices" | sort -r | awk -v z="$zone" '$2 == z {print $3; exit}')" "$zone"
    done | awk '{if (NF == 1) print "99", $1; else print}' | sort -n
}

# The user data of the launch template: a script that sets the timer, installs, checks out the commit, finds
# wall-node.sh in the checkout and starts it in tmux.  If any of that fails, or the tmux session is gone a few seconds
# later, it ends the run with the reason, so that the group does not launch the same failure again and again.
write_user_data() {
    local file="$1" timer_minutes="$2" commit="$3"
    {
        printf '#!/bin/bash\n'
        printf '# The user data of the launch template %s (solver/tools/cloud/wall.sh).\n' "$NAME"
        printf 'export AWS_DEFAULT_REGION=%s S3_DEST=%s ASG=%s\n' "$AWS_DEFAULT_REGION" "$S3_DEST" "$NAME"
        printf '# The timer: no machine lives longer than the whole budget and two hours, whatever the scripts do.\n'
        printf 'shutdown -h +%s\n' "$timer_minutes"
        printf 'retry() { local n=0; until "$@"; do n=$((n + 1)); [ "$n" -lt 5 ] || return 1; sleep 15; done; }\n'
        printf 'fail() {\n'
        printf '    echo "boot failed: $*"\n'
        printf '    printf "time=%%s\\nreason=%%s\\n" "$(date +%%s)" "boot failed: $*" |\n'
        printf '        aws s3 cp - "$S3_DEST/run/ended"\n'
        printf '    aws autoscaling set-desired-capacity --auto-scaling-group-name "$ASG" --desired-capacity 0\n'
        printf '    shutdown -h now\n'
        printf '    exit 1\n'
        printf '}\n'
        printf 'clone() {\n'
        printf '    rm -rf /home/ec2-user/taxman\n'
        printf '    sudo -u ec2-user git clone https://github.com/bvchess/taxman.git /home/ec2-user/taxman\n'
        printf '}\n'
        printf 'retry dnf install -y git tmux || fail "dnf install git tmux"\n'
        printf 'retry clone || fail "git clone"\n'
        printf 'sudo -u ec2-user git -C /home/ec2-user/taxman checkout %s ||\n' "$commit"
        printf '    fail "git checkout %s"\n' "$commit"
        # A commit from before the restructure of 2026-10-18 has the script in rust/ and takes the same environment.
        printf 'node=solver/tools/cloud/wall-node.sh\n'
        printf '[ -f /home/ec2-user/taxman/$node ] || node=rust/tools/cloud/wall-node.sh\n'
        printf '[ -f /home/ec2-user/taxman/$node ] ||\n'
        printf '    fail "wall-node.sh is in neither solver/tools/cloud nor rust/tools/cloud at %s"\n' "$commit"
        printf 'sudo -u ec2-user -i env AWS_DEFAULT_REGION=%s S3_DEST=%s ASG=%s \\\n' "$AWS_DEFAULT_REGION" "$S3_DEST" \
            "$NAME"
        printf '    tmux new-session -d -s tx -c /home/ec2-user/taxman "$node" ||\n'
        printf '    fail "tmux"\n'
        # tmux says success for a command that does not run: the session has to be alive a few seconds later.
        printf 'sleep 5\n'
        printf 'sudo -u ec2-user -i tmux has-session -t tx || fail "wall-node did not start"\n'
    } >"$file"
}

# The launch template's data, as JSON.
write_template_data() {
    local file="$1" user_data="$2" type="$3" sg="$4" volume="$5"
    {
        printf '{"ImageId":"%s","InstanceType":"%s","IamInstanceProfile":{"Name":"%s"},"SecurityGroupIds":["%s"],' \
            "$IMAGE" "$type" "$ROLE" "$sg"
        printf '"MetadataOptions":{"HttpTokens":"required","HttpEndpoint":"enabled"},'
        printf '"BlockDeviceMappings":[{"DeviceName":"/dev/xvda","Ebs":{"VolumeSize":%s,"VolumeType":"gp3",' "$volume"
        printf '"DeleteOnTermination":true}}],"InstanceInitiatedShutdownBehavior":"terminate","UserData":"%s",' \
            "$(base64 <"$user_data" | tr -d '\n')"
        printf '"TagSpecifications":[{"ResourceType":"instance","Tags":[{"Key":"Name","Value":"%s"},' "$NAME"
        printf '{"Key":"Project","Value":"taxman"},{"Key":"Purpose","Value":"wall"}]},'
        printf '{"ResourceType":"volume","Tags":[{"Key":"Project","Value":"taxman"},'
        printf '{"Key":"Purpose","Value":"wall"}]}]}\n'
    } >"$file"
}

# The group's mixed instances policy: spot only, the lowest price of the zones, one instance type.
write_mixed_policy() {
    local file="$1" type="$2" spot_max="$3"
    {
        printf '{"LaunchTemplate":{"LaunchTemplateSpecification":{"LaunchTemplateName":"%s",' "$NAME"
        printf '"Version":"$Latest"},'
        printf '"Overrides":[{"InstanceType":"%s"}]},"InstancesDistribution":{"OnDemandBaseCapacity":0,' "$type"
        printf '"OnDemandPercentageAboveBaseCapacity":0,"SpotAllocationStrategy":"lowest-price","SpotInstancePools":1,'
        printf '"SpotMaxPrice":"%s"}}\n' "$spot_max"
    } >"$file"
}

# The config object: name=value lines, the settings that were given (wall-node.sh takes the names it knows, each as the
# rest of its line; PRICE is for status).
write_config() {
    local file="$1" name
    : >"$file"
    for name in COMMIT INSTANCE_TYPE CHAIN_TO WINDOW GAME_HOURS BUDGET_HOURS THREADS EXTRA_ARGS CHECKPOINT_INTERVAL \
        SAVE_EVERY LEDGER_MARGIN MAX_INSTANCES PRICE RELEASE_TESTS REQUIRE_CACHED; do
        [ -z "${!name}" ] || printf '%s=%s\n' "$name" "${!name}" >>"$file"
    done
    printf 'RUN_UNTIL=%s\n' "$UNTIL_EPOCH" >>"$file"
}

launch() {
    COMMIT="${COMMIT:-}"
    UNTIL="${UNTIL:-}"
    BUDGET_HOURS="${BUDGET_HOURS:-74}"
    GAME_HOURS="${GAME_HOURS:-24}"
    CHAIN_TO="${CHAIN_TO:-8191}"
    WINDOW="${WINDOW:-4}"
    THREADS="${THREADS:-}"
    INSTANCE_TYPE="${INSTANCE_TYPE:-c8g.16xlarge}"
    ZONES="${ZONES:-}"
    SPOT_MAX="${SPOT_MAX:-1.0}"
    PRICE="${PRICE:-0.676}"
    VOLUME_GB="${VOLUME_GB:-40}"
    EXTRA_ARGS="${EXTRA_ARGS:-}"
    LEDGER_MARGIN="${LEDGER_MARGIN:-300}"
    MAX_INSTANCES="${MAX_INSTANCES:-20}"
    SAVE_EVERY="${SAVE_EVERY:-}"
    CHECKPOINT_INTERVAL="${CHECKPOINT_INTERVAL:-}"
    RELEASE_TESTS="${RELEASE_TESTS:-}"
    REQUIRE_CACHED="${REQUIRE_CACHED:-}"
    local continuing=0 number
    while [ "$#" -gt 0 ]; do
        case "$1" in
            --continue)
                continuing=1
                shift
                continue
                ;;
            -h | --help)
                usage
                exit 0
                ;;
        esac
        [ "$#" -ge 2 ] || die "launch: $1 needs a value (see the top of $0)"
        case "$1" in
            --commit) COMMIT="$2" ;;
            --until) UNTIL="$2" ;;
            --budget-hours) BUDGET_HOURS="$2" ;;
            --game-hours) GAME_HOURS="$2" ;;
            --chain-to) CHAIN_TO="$2" ;;
            --window) WINDOW="$2" ;;
            --threads) THREADS="$2" ;;
            --instance-type) INSTANCE_TYPE="$2" ;;
            --zones) ZONES="$2" ;;
            --spot-max) SPOT_MAX="$2" ;;
            --price) PRICE="$2" ;;
            --volume-gb) VOLUME_GB="$2" ;;
            --extra-args) EXTRA_ARGS="$2" ;;
            --margin) LEDGER_MARGIN="$2" ;;
            --max-instances) MAX_INSTANCES="$2" ;;
            --save-every) SAVE_EVERY="$2" ;;
            --checkpoint-interval) CHECKPOINT_INTERVAL="$2" ;;
            *) die "launch: unknown option $1 (see the top of $0)" ;;
        esac
        shift 2
    done
    if [ "${#COMMIT}" != 40 ] || [ -n "$(printf '%s' "$COMMIT" | tr -d '0-9a-f')" ]; then
        die "launch: --commit must be a full commit id of 40 hex digits (git rev-parse), pushed to origin"
    fi
    [ -n "$UNTIL" ] || die "launch: --until DATE is needed: the hard date after which the run stops whatever its state"
    UNTIL_EPOCH="$(to_epoch "$UNTIL")" || die "launch: cannot read the date $UNTIL (YYYY-MM-DD or YYYY-MM-DDTHH:MM:SSZ)"
    [ "$UNTIL_EPOCH" -gt "$(date +%s)" ] || die "launch: the date $UNTIL is past"
    for number in "$BUDGET_HOURS" "$GAME_HOURS" "$SPOT_MAX" "$PRICE" "$LEDGER_MARGIN" "$MAX_INSTANCES" "$VOLUME_GB" \
        "$CHAIN_TO" "$WINDOW"; do
        is_number "$number" || die "launch: \"$number\" is not a number"
    done
    for number in "$SAVE_EVERY" "$CHECKPOINT_INTERVAL"; do
        [ -z "$number" ] || is_number "$number" || die "launch: \"$number\" is not a number"
    done
    [ -r "$here/wall-policy.json" ] || die "launch: $here/wall-policy.json is missing"
    if git -C "$here" cat-file -e "$COMMIT^{commit}" 2>/dev/null; then
        [ -n "$(git -C "$here" branch -r --contains "$COMMIT" 2>/dev/null)" ] ||
            log "warning: $COMMIT is on no remote branch of this checkout (git fetch? git push?):" \
                "the machines clone it from GitHub"
    else
        log "warning: $COMMIT is unknown to this checkout: the machines clone it from GitHub"
    fi

    need_account
    mkdir -p "$work"

    # What exists already: a run is one at a time.
    local found
    found="$(read_aws autoscaling describe-auto-scaling-groups --auto-scaling-group-names "$NAME" \
        --query 'AutoScalingGroups[].AutoScalingGroupName' --output text)" || die "cannot read the groups"
    [ -z "$found" ] || die "the group $NAME exists: wall.sh cleanup first (or wall.sh stop, status)"
    found="$(read_aws ec2 describe-launch-templates --filters "Name=launch-template-name,Values=$NAME" \
        --query 'LaunchTemplates[].LaunchTemplateName' --output text)" || die "cannot read the launch templates"
    [ -z "$found" ] || die "the launch template $NAME exists: wall.sh cleanup first"
    if has_object config; then
        if [ "$continuing" = 1 ]; then
            log "going on with the run whose state is in $S3_DEST (its ledger, results and checkpoints are kept)"
        else
            die "$S3_DEST/run/config exists: that run's state is there; use another RUN, or --continue to go on with it"
        fi
    elif [ "$continuing" = 1 ]; then
        die "--continue, but $S3_DEST/run/config does not exist: there is no run to go on with"
    fi
    read_aws iam get-instance-profile --instance-profile-name "$ROLE" --query 'InstanceProfile.Roles[].RoleName' \
        --output text >/dev/null || die "the instance profile $ROLE does not exist (solver/docs/runbook/launch.md)"

    # Where: the zones, cheapest first, and their subnets in the default VPC.
    local vpc subnets zone subnet
    if [ -z "$ZONES" ]; then
        local ranked
        ranked="$(zones_by_price "$INSTANCE_TYPE")" || exit 1
        log "zones by the spot price of the last day (dollars an hour, zone):"
        printf '%s\n' "$ranked" | sed 's/^/    /' >&2
        ZONES="$(printf '%s\n' "$ranked" | awk '{print $2}' | tr '\n' ' ')"
    fi
    vpc="$(read_aws ec2 describe-vpcs --filters Name=is-default,Values=true --query 'Vpcs[0].VpcId' --output text)" ||
        die "cannot read the default VPC"
    subnets=""
    for zone in $ZONES; do
        subnet="$(read_aws ec2 describe-subnets \
            --filters "Name=vpc-id,Values=$vpc" "Name=availability-zone,Values=$zone" Name=default-for-az,Values=true \
            --query 'Subnets[0].SubnetId' --output text)" || die "cannot read the subnet of $zone"
        { [ -n "$subnet" ] && [ "$subnet" != None ]; } || die "no default subnet in $zone"
        subnets="${subnets:+$subnets,}$subnet"
    done
    [ -n "$subnets" ] || die "no zone for $INSTANCE_TYPE"

    # The files the commands take, shown before they are used.
    local timer_minutes
    timer_minutes="$(awk -v h="$BUDGET_HOURS" 'BEGIN {printf "%d", (h + 2) * 60}')"
    write_user_data "$work/user-data.sh" "$timer_minutes" "$COMMIT"
    write_config "$work/config"
    write_mixed_policy "$work/mixed.json" "$INSTANCE_TYPE" "$SPOT_MAX"
    printf '\n--- the user data of the launch template ---\n' >&2
    cat "$work/user-data.sh" >&2
    printf -- '--- the config of the run, %s ---\n' "$S3_DEST/run/config" >&2
    cat "$work/config" >&2
    printf -- '--- the policy for the role %s (%s) ---\n' "$ROLE" "$here/wall-policy.json" >&2
    cat "$here/wall-policy.json" >&2
    printf -- '--- the mixed instances policy of the group ---\n' >&2
    cat "$work/mixed.json" >&2
    printf -- '---\n\n' >&2
    local dollars
    dollars="$(awk -v h="$BUDGET_HOURS" -v p="$PRICE" 'BEGIN {printf "%.2f", h * p}')"
    log "budget $BUDGET_HOURS instance-hours (about \$$dollars at \$$PRICE an hour), wall $GAME_HOURS hours," \
        "until $UNTIL, zones: $ZONES"

    # 1. The run's config (going on, with no end and no stop).
    if [ "$continuing" = 1 ]; then
        has_object ended && change_aws s3 rm "$S3_DEST/run/ended"
        has_object stop && change_aws s3 rm "$S3_DEST/run/stop"
        # The machines of the run so far are not a boot loop of this one.
        change_aws s3 rm "$S3_DEST/run/boots/" --recursive --only-show-errors
    fi
    change_aws s3 cp "$work/config" "$S3_DEST/run/config" --only-show-errors || die "cannot write the config"
    # 2. The permission the machines need at the end.
    change_aws iam put-role-policy --role-name "$ROLE" --policy-name "$NAME" \
        --policy-document "file://$here/wall-policy.json" || die "cannot put the policy"
    # 3. The security group.
    local sg
    sg="$(change_aws_value sg-DRYRUN ec2 create-security-group --group-name "$NAME" --vpc-id "$vpc" \
        --description 'taxman wall run, no inbound' --tag-specifications "ResourceType=security-group,Tags=[$TAGS]" \
        --query GroupId --output text)" || die "cannot create the security group"
    # 4. The launch template.
    write_template_data "$work/template.json" "$work/user-data.sh" "$INSTANCE_TYPE" "$sg" "$VOLUME_GB"
    change_aws ec2 create-launch-template --launch-template-name "$NAME" \
        --version-description "wall run $RUN, commit $COMMIT" --launch-template-data "file://$work/template.json" \
        --tag-specifications "ResourceType=launch-template,Tags=[$TAGS]" --query LaunchTemplate.LaunchTemplateId \
        --output text || die "cannot create the launch template (wall.sh cleanup --force removes what was made)"
    # 5. The group, at capacity 0 for now: one instance at most, no capacity rebalancing, EC2 health checks.
    change_aws autoscaling create-auto-scaling-group --auto-scaling-group-name "$NAME" \
        --mixed-instances-policy "file://$work/mixed.json" --min-size 0 --max-size 1 --desired-capacity 0 \
        --vpc-zone-identifier "$subnets" --no-capacity-rebalance --health-check-type EC2 --health-check-grace-period 0 \
        --tags "ResourceId=$NAME,ResourceType=auto-scaling-group,Key=Project,Value=taxman,PropagateAtLaunch=true" \
        "ResourceId=$NAME,ResourceType=auto-scaling-group,Key=Purpose,Value=wall,PropagateAtLaunch=true" \
        "ResourceId=$NAME,ResourceType=auto-scaling-group,Key=Name,Value=$NAME,PropagateAtLaunch=true" ||
        die "cannot create the group (wall.sh cleanup --force removes what was made)"
    # 6. The backstop that depends on no script: at the date and two hours, capacity 0.
    change_aws autoscaling put-scheduled-update-group-action --auto-scaling-group-name "$NAME" \
        --scheduled-action-name "$NAME-date" --start-time "$(epoch_date $((UNTIL_EPOCH + 7200)))" \
        --min-size 0 --max-size 0 --desired-capacity 0 ||
        die "cannot put the scheduled action (wall.sh cleanup --force removes what was made)"
    # 7. Go.
    change_aws autoscaling set-desired-capacity --auto-scaling-group-name "$NAME" --desired-capacity 1 ||
        die "cannot start the first machine"
    printf '%s\n' "launched.  The first machine starts in a minute or two and builds for some minutes.  Then:" \
        "    $0 status          where the run is" "    $0 stop [--now]    end it" \
        "    $0 cleanup         delete what launch made, once the run has ended" >&2
}

# --- status ------------------------------------------------------------------------------------------------------

# The awk function commas(x) that the awk programs of status include (as "$AWK_COMMAS"): the whole number x with a comma
# between each three digits, 6971841261 as 6,971,841,261; any other text (a `-`) as it is.  The one definition of it:
# commas, below, runs it for the shell.
AWK_COMMAS='
    function commas(x,    s, out) {
        s = x ""
        if (s !~ /^[0-9]+$/) return s
        while (length(s) > 3) {
            out = "," substr(s, length(s) - 2) out
            s = substr(s, 1, length(s) - 3)
        }
        return s out
    }
'

# The whole number $1 with its thousands separators (AWK_COMMAS), not by printf's ' flag, which depends on the locale.
commas() {
    awk -v x="$1" "$AWK_COMMAS"'BEGIN { print commas(x) }'
}

# The last line of each of the COUNT highest games up to THROUGH among the lines of recorded games on stdin
# (logs/chain.out), the lowest game first.  A replacement machine may print a game again after an interruption: the last
# line of a game is the one that counts.
recorded_games() {
    awk -v count="$1" -v through="$2" '
    { n = substr($1, 3) + 0; if (n <= through) line[n] = $0 }
    END {
        for (n = through; n >= 1 && shown < count; n--) if (n in line) game[++shown] = n
        for (i = shown; i >= 1; i--) print line[game[i]]
    }'
}

# The games, a line each: the recorded games, then those in flight.  The text on stdin is the lines of the recorded
# games (recorded_games), then the progress lines of the solver (logs/chain.err).  A recorded game's line, `n=4864
# score=7437375   search  nodes=1133462769   4636.026 s  optimal unknown  proof=exhausted`, gives its nodes and its
# seconds: its setup, its search from the start (waits included) and its conclusion, in the run that recorded it, so
# they go under `elapsed` (less the runs of a search before an interruption); it has no wall clock and no best sum of
# promotions.  A game in flight whose search has started prints one progress line every PROGRESS seconds, `n=3930
# 1h45m00s nodes 1086013550 (242.8k/s) best 341541 queue ...` (the time since the search began, waits included; the
# nodes so far; the nodes a second in the last interval, or `done`, `optimal`, `stopped`, `failed` or `panicked` in the
# last line; the best sum of promotions so far), and nothing else says where a game is: a game with no recent line has
# no state to tell (a setup, a prime or repeated game, a search not started).  Arguments: the first game in flight, how
# many games are in flight, and the games' wall clocks (ckpt/nNNNN.clock) as "n seconds n seconds ...".  A line is
# recent unless three lines of another game in flight follow it (a game that still searches prints once an interval).
# The state of a game in flight is told from its last line:
#   running      the nodes grow (1000 a second or more in the last interval)
#   suspended    the search has started, but the threads are another game's, and the nodes do not grow
#   search done  the search has ended (`done` or `optimal`) and the game is not recorded yet: the oldest game is being
#                recorded, any other waits for the games before it
games_table() {
    awk -v first="$1" -v flight="$2" -v clocks="$3" "$AWK_COMMAS"'
    function duration(s) {
        if (s < 60) return sprintf("%.1fs", s)
        if (s < 3600) return sprintf("%dm%02ds", s / 60, s % 60)
        return sprintf("%dh%02dm%02ds", s / 3600, s / 60 % 60, s % 60)
    }
    # Nodes a second, from the rate of a line: 0, 1, 236.5k, 1.20M, ...
    function rate(text,    unit) {
        sub("/s$", "", text)
        unit = substr(text, length(text))
        text = text + 0
        if (unit == "k") text *= 1e3
        else if (unit == "M") text *= 1e6
        else if (unit == "G") text *= 1e9
        return text
    }
    function row(n, own, elapsed, nodes, best, state) {
        printf "  %5s  %-10s  %-10s  %14s  %9s  %s\n", n, own, elapsed, commas(nodes), commas(best), state
    }
    BEGIN {
        first += 0
        last = first + flight - 1
        count = split(clocks, c, " ")
        for (i = 1; i < count; i += 2) held[c[i] + 0] = c[i + 1] + 0
        row("n", "own search", "elapsed", "nodes", "best", "state")
    }
    # A recorded game, a row at once, so that the recorded games come above those in flight.  Its state is its score,
    # its method, its proof (the text after `proof=` up to the next two spaces) and `optimal ok`, if the solver found
    # the game in its table of optimal games.
    $1 ~ /^n=[0-9]+$/ && $2 ~ /^score=[0-9]+$/ && $4 ~ /^nodes=[0-9]+$/ && $6 == "s" {
        detail = $3
        where = index($0, "  proof=")
        if (where) {
            proof = substr($0, where + 8)
            sub(/  .*/, "", proof)
            detail = detail ", " proof
        }
        if ($7 == "optimal" && $8 != "unknown") detail = detail ", optimal " $8
        row(substr($1, 3), "-", duration($5 + 0), substr($4, 7), "-",
            "recorded, score " commas(substr($2, 7)) " (" detail ")")
    }
    $1 ~ /^n=[0-9]+$/ && $3 == "nodes" && $4 ~ /^[0-9]+$/ && $5 ~ /^\(.*\)$/ && $6 == "best" && $8 == "queue" {
        n = substr($1, 3) + 0
        how = substr($5, 2, length($5) - 2)
        lines++
        game[lines] = n
        finished[lines] = (how !~ /\/s$/)
        at[n] = lines
        elapsed[n] = $2
        nodes[n] = $4
        best[n] = $7
        last_how[n] = how
    }
    END {
        for (g = first; g <= last; g++) {
            own = (g in held) ? duration(held[g]) : "-"
            how = (g in at) ? last_how[g] : ""
            searching = (how != "done" && how != "optimal")
            stale = 0
            if (g in at && searching) {
                split("", printed)
                for (i = at[g] + 1; i <= lines; i++) {
                    h = game[i]
                    if (h != g && h >= first && h <= last && !finished[i] && ++printed[h] >= 3) stale = 1
                }
            }
            if (!(g in at) || stale) {
                row(g, own, "-", "-", "-", "no recent progress line")
                continue
            }
            if (!searching) {
                state = (g == first) ? "search done, being recorded" : "search done, waiting for the game before"
            } else if (how == "stopped") {
                state = "search stopped, its checkpoint kept"
            } else if (how == "failed" || how == "panicked") {
                state = "search " how
            } else if (rate(how) < 1000) {
                state = "suspended"
            } else {
                state = how
                sub("/s$", " nodes/s", state)
                state = "running, " state
            }
            row(g, own, elapsed[g], nodes[g], best[g], state)
        }
    }'
}

# The group's capacity and instances, a line each, from the text of describe-auto-scaling-groups on stdin: the desired
# capacity, then `ID ZONE STATE` (tab-separated) for each instance; `None` is a group that does not exist.
group_lines() {
    awk -F '\t' -v name="$NAME" '
    NR == 1 && ($0 == "None" || $0 == "") { missing = 1; exit }
    NR == 1 { printf "  desired capacity %s\n", $0; next }
    NF >= 3 { printf "  instance %s in %s, %s\n", $1, $2, $3; instances++ }
    END {
        if (missing || NR == 0) printf "  no group %s\n", name
        else if (!instances) printf "  no instance\n"
    }'
}

# The line above the table of games: the RECORDED games it shows, and the IN_FLIGHT games in flight (WINDOW is the most
# at a time, and the wall of WALL_HOURS is what "own search" counts toward).  No line for a table with no game.
table_heading() {
    local recorded="$1" in_flight="$2" window="$3" wall_hours="$4" what=""
    case "$recorded" in
        0) ;;
        1) what="the last game recorded" ;;
        *) what="the last $recorded games recorded" ;;
    esac
    if [ "$in_flight" -gt 0 ]; then
        if [ -n "$what" ]; then what="$what and the $in_flight in flight"; else what="games in flight"; fi
        what="$what (window $window; \"own search\" counts toward the wall of $wall_hours hours)"
    fi
    [ -z "$what" ] || printf '  %s:\n' "$what"
}

# The seconds on the clock of a game (ckpt/nNNNN.clock) in S3, for the file NAME.
s3_clock() {
    s3_text "ckpt/$1" | awk '{print $1}'
}

# The progress lines of the solver in S3 (logs/chain.err).
s3_progress() {
    s3_text logs/chain.err
}

# The games of the chain, from what each path reads.  Arguments: the game recorded through and its score (both empty for
# no game), the window, the last game of the chain and the wall in hours, the lines of the recorded games
# (logs/chain.out), the command that prints the progress lines of the solver (logs/chain.err; empty if there are none to
# read) and the note for the games in flight when there are none, the command that prints the seconds on the clock of
# the file NAME (ckpt/nNNNN.clock) and the names of those files.  It prints the line of the game recorded through, the
# heading, and the table of the last SHOW_RECORDED games recorded and the games in flight after the game recorded
# through.
chain_games() {
    local recorded="$1" score="$2" window="$3" chain_to="$4" wall_hours="$5" lines="$6" progress_fn="$7" note="$8"
    local clock_fn="$9" clock_names="${10}" first_n last_n in_flight shown shown_count name n seconds clocks=""
    if [ -n "$recorded" ]; then
        printf '  recorded through %s (score %s)\n' "$recorded" "$(commas "$score")"
    else
        printf '  recorded through: no game yet\n'
        recorded=0
    fi
    first_n=$((recorded + 1))
    last_n=$((recorded + window))
    [ "$last_n" -le "$chain_to" ] || last_n="$chain_to"
    in_flight=$((last_n - first_n + 1))
    [ "$in_flight" -gt 0 ] || in_flight=0
    for name in $clock_names; do
        n="${name#n}"
        n=$((10#${n%.clock}))
        if [ "$n" -ge "$first_n" ] && [ "$n" -le "$last_n" ]; then
            seconds="$("$clock_fn" "$name")"
            [ -z "$seconds" ] || clocks="$clocks $n $seconds"
        fi
    done
    shown="$(printf '%s\n' "$lines" | recorded_games "$SHOW_RECORDED" "$recorded")"
    shown_count="$(printf '%s\n' "$shown" | grep -c .)"
    table_heading "$shown_count" "$in_flight" "$window" "$wall_hours"
    if [ "$in_flight" -gt 0 ] && [ -n "$progress_fn" ]; then
        { printf '%s\n' "$shown"; "$progress_fn"; } | games_table "$first_n" "$in_flight" "$clocks"
    else
        [ "$shown_count" -eq 0 ] || printf '%s\n' "$shown" | games_table "$first_n" 0 ""
        if [ "$in_flight" -gt 0 ]; then
            printf '  %s\n' "$note"
        else
            printf '  no game in flight: the range to %s is recorded\n' "$chain_to"
        fi
    fi
}

status() {
    need_run
    local config ended ledger hours start updated budget price until_epoch boots window chain_to wall_hours group
    config="$(s3_text run/config)" || die "no run $RUN in $S3_DEST (nothing at $S3_DEST/run/config)"
    budget="$(printf '%s\n' "$config" | field BUDGET_HOURS)"
    price="$(printf '%s\n' "$config" | field PRICE)"
    until_epoch="$(printf '%s\n' "$config" | field RUN_UNTIL)"
    window="$(printf '%s\n' "$config" | field WINDOW)"
    window="${window:-4}"
    chain_to="$(printf '%s\n' "$config" | field CHAIN_TO)"
    chain_to="${chain_to:-8191}"
    wall_hours="$(printf '%s\n' "$config" | field GAME_HOURS)"
    printf 'run %s  (%s)\n' "$RUN" "$S3_DEST"
    if [ "$RUN_PICKED" = 1 ]; then printf '  the run in progress (RUN was not set)\n'; fi
    printf '  account %s, region %s\n' "$ACCOUNT" "$AWS_DEFAULT_REGION"
    printf '  commit %s, %s, wall %s hours, window %s, budget %s instance-hours, until %s\n' \
        "$(printf '%s\n' "$config" | field COMMIT | cut -c1-12)" "$(printf '%s\n' "$config" | field INSTANCE_TYPE)" \
        "$wall_hours" "$window" "$budget" "$(epoch_local "$until_epoch")"

    ended="$(s3_text run/ended)"
    if [ -n "$ended" ]; then
        printf '  ENDED %s: %s\n' "$(epoch_local "$(printf '%s\n' "$ended" | field time)")" \
            "$(printf '%s\n' "$ended" | field reason)"
    elif has_object stop; then
        printf '  stop requested: the machine is stopping the chain, then ends the run\n'
    else
        printf '  not ended\n'
    fi

    ledger="$(s3_text run/ledger)"
    if [ -n "$ledger" ]; then
        start="$(printf '%s\n' "$ledger" | field start)"
        hours="$(printf '%s\n' "$ledger" | field hours)"
        updated="$(printf '%s\n' "$ledger" | field updated)"
        awk -v start="$start" -v now="$(date +%s)" -v hours="$hours" -v budget="$budget" -v price="$price" \
            -v began="$(epoch_local "$start")" -v seen="$(epoch_local "$updated")" 'BEGIN {
            printf "  the run began %s, %.2f hours of wall clock ago\n", began, (now - start) / 3600
            printf "  instance-hours %.2f of %s (%.0f%%), counted to %s\n", hours, budget, 100 * hours / budget, seen
            printf "  spend about $%.2f of $%.2f, at $%s an hour\n", hours * price, budget * price, price }'
    else
        printf '  no ledger yet: no machine has started the chain\n'
    fi
    boots="$(s3_ls "$S3_DEST/run/boots/" | wc -l | tr -d ' ')"
    printf '  machines started so far: %s\n' "$boots"

    printf 'the group %s\n' "$NAME"
    if group="$(read_aws_quiet autoscaling describe-auto-scaling-groups --auto-scaling-group-names "$NAME" \
        --output text --query \
        'AutoScalingGroups[0].[DesiredCapacity,Instances[].[InstanceId,AvailabilityZone,LifecycleState]]')"; then
        printf '%s\n' "$group" | group_lines
    else
        printf '  cannot read the group %s\n' "$NAME"
    fi

    # The chain is as old as the last save: the newest object of results/, logs/ and ckpt/ (the ledger and the rest of
    # run/ are written on their own).
    printf 'the chain (as of the last save to S3)\n'
    local listing prefix saved saved_epoch lines out recorded score progress_fn=""
    prefix="${S3_DEST#s3://$BUCKET/}"
    listing="$(s3_ls --recursive "$S3_DEST/")"
    saved="$(printf '%s\n' "$listing" | awk -v p="$prefix" '
        index($4, p "/results/") == 1 || index($4, p "/logs/") == 1 || index($4, p "/ckpt/") == 1 {
            print $1 "T" $2 "Z" }' | sort | tail -n 1)"
    if [ -n "$saved" ]; then
        saved_epoch="$(to_epoch "$saved")"
        printf '  last save to S3: %s, %s ago\n' "$(epoch_local "$saved_epoch")" \
            "$(span_text $(($(date +%s) - saved_epoch)))"
    else
        printf '  nothing has been saved to S3 yet\n'
    fi
    # The solver prints a line to chain.out for each game it records: the last is the last game of the results.
    lines="$(s3_text logs/chain.out | grep '^n=' | grep -v ' incomplete$')"
    out="$(printf '%s\n' "$lines" | tail -n 1)"
    recorded="$(printf '%s\n' "$out" | sed -n 's/^n=\([0-9]*\) .*/\1/p')"
    score="$(printf '%s\n' "$out" | sed -n 's/.* score=\([0-9]*\) .*/\1/p')"
    # The machine saves chain.err with its first save: until then it is not in S3, and that is no error.
    if printf '%s\n' "$listing" | awk -v f="$prefix/logs/chain.err" '$4 == f { found = 1 } END { exit !found }'; then
        progress_fn=s3_progress
    fi
    chain_games "$recorded" "$score" "$window" "$chain_to" "$wall_hours" "$lines" "$progress_fn" \
        "no progress lines saved yet" s3_clock "$(printf '%s\n' "$listing" | awk -v p="$prefix/ckpt/" \
        'index($4, p) == 1 && substr($4, length(p) + 1) ~ /^n[0-9]+[.]clock$/ {print substr($4, length(p) + 1)}')"
}

# The script that the host runs (bash -s), one ssh call for all that status --host shows: sections, each after a line
# @@NAME, of the state of a chain in ~/taxman-work.  The results file's last lines are enough to find the last game
# completed, and the last 3000 lines of chain.err to tell the games in flight apart.
host_script() {
    cat <<'EOF'
work="$HOME/taxman-work"
changed() { stat -c %Y "$1" 2>/dev/null || stat -f %m "$1" 2>/dev/null; }
echo @@now
date +%s
echo @@uptime
uptime
echo @@session
if tmux has-session -t tx 2>/dev/null; then echo yes; else echo no; fi
echo @@solver
if ps -eo args | grep -q '[t]axman solve'; then echo yes; else echo no; fi
echo @@outcome
cat "$work/chain.outcome" 2>/dev/null
echo @@start
grep -E ' chain [0-9]+-[0-9]+; threads: ' "$work/logs/chain.log" 2>/dev/null | tail -n 1
echo @@errtime
changed "$work/logs/chain.err"
echo @@err
tail -n 3000 "$work/logs/chain.err" 2>/dev/null
echo @@result
tail -n 100 "$work/results/strict.jsonl" 2>/dev/null | grep '"completed":true' | tail -n 1 |
    grep -o '"n":[0-9]*,"score":[0-9]*' | head -n 1
echo @@out
grep '^n=' "$work/logs/chain.out" 2>/dev/null | grep -v ' incomplete$'
echo @@clocks
for f in "$work"/ckpt/n*.clock; do
    [ -f "$f" ] && echo "$(basename "$f") $(awk '{print $1; exit}' "$f")"
done
true
EOF
}

# The section NAME of the host's answer (HOST_DATA), without its marker.
host_section() {
    printf '%s\n' "$HOST_DATA" | awk -v marker="@@$1" '$0 == marker { on = 1; next } /^@@/ { on = 0 } on'
}

# The seconds on the clock of a game (ckpt/nNNNN.clock) on the host, for the file NAME, from HOST_CLOCKS: the section
# clocks of the answer.
host_clock() {
    printf '%s\n' "$HOST_CLOCKS" | awk -v name="$1" '$1 == name { print $2; exit }'
}

# The progress lines of the solver on the host (logs/chain.err).
host_progress() {
    host_section err
}

# status for a chain on another machine: what it shows is in ~/taxman-work there, read over ssh, and no AWS.
status_host() {
    local now session solver running outcome start window chain_to wall_hours changed result recorded score
    local progress_fn=""
    HOST_DATA="$(host_script | "$SSH_CLI" -o BatchMode=yes -o ConnectTimeout=10 "$WALL_HOST" 'bash -s')" ||
        die "cannot read the chain on $WALL_HOST over ssh"
    now="$(host_section now)"
    [ -n "$now" ] || die "$WALL_HOST did not answer as expected: $(printf '%s\n' "$HOST_DATA" | head -n 1)"
    printf 'host %s\n' "$WALL_HOST"
    printf '  %s\n' "$(host_section uptime | sed 's/^ *//')"
    session="$(host_section session)"
    solver="$(host_section solver)"
    running="is NOT running"
    if [ "$session" = yes ] && [ "$solver" = yes ]; then running="is running"; fi
    if [ "$session" = yes ]; then session="the tmux session tx exists"; else session="no tmux session tx"; fi
    if [ "$solver" = yes ]; then solver="a taxman solve process is alive"; else solver="no taxman solve process"; fi
    printf '  the chain %s (%s, %s)\n' "$running" "$session" "$solver"
    outcome="$(host_section outcome)"
    if [ -n "$outcome" ]; then printf '  ENDED: %s\n' "$outcome"; fi

    start="$(host_section start)"
    window="$(printf '%s\n' "$start" | sed -n 's/.* window \([0-9][0-9]*\);.*/\1/p')"
    window="${window:-4}"
    chain_to="$(printf '%s\n' "$start" | sed -n 's/.* chain 1-\([0-9][0-9]*\);.*/\1/p')"
    chain_to="${chain_to:-8191}"
    wall_hours="$(printf '%s\n' "$start" | sed -n 's/.* wall: \([^ ]*\) hours;.*/\1/p')"
    if [ -n "$start" ]; then
        printf '  chain 1-%s, window %s, wall %s hours\n' "$chain_to" "$window" "$wall_hours"
    else
        printf '  no line of chain.log starts the chain yet\n'
    fi
    changed="$(host_section errtime)"
    if [ -n "$changed" ]; then
        printf '  last progress line: %s, %s ago\n' "$(epoch_local "$changed")" "$(span_text $((now - changed)))"
        progress_fn=host_progress
    else
        printf '  no progress line yet: no logs/chain.err\n'
    fi
    # The results file holds the whole chain; chain.out only the games of this machine's run.
    result="$(host_section result)"
    recorded="$(printf '%s\n' "$result" | sed -n 's/^"n":\([0-9]*\),.*/\1/p')"
    score="$(printf '%s\n' "$result" | sed -n 's/.*"score":\([0-9]*\)$/\1/p')"
    HOST_CLOCKS="$(host_section clocks)"
    chain_games "$recorded" "$score" "$window" "$chain_to" "$wall_hours" "$(host_section out)" "$progress_fn" \
        "no progress lines yet" host_clock "$(printf '%s\n' "$HOST_CLOCKS" | awk '{print $1}')"
}

# --- stop --------------------------------------------------------------------------------------------------------

stop() {
    local now=0
    [ "${1:-}" = --now ] && now=1
    need_account
    mkdir -p "$work"
    if [ "$now" = 1 ]; then
        # The end is written first, so that no machine that starts meanwhile takes up the run.
        printf 'time=%s\nreason=%s\n' "$(date +%s)" "stopped at once by wall.sh stop --now" >"$work/ended"
        has_object ended || change_aws s3 cp "$work/ended" "$S3_DEST/run/ended" --only-show-errors ||
            die "cannot write the end"
        change_aws autoscaling set-desired-capacity --auto-scaling-group-name "$NAME" --desired-capacity 0 ||
            die "cannot set the desired capacity"
        log "the machine is being terminated: what it did since its last save (at most SAVE_EVERY seconds) is lost"
    else
        printf '%s\n' "stop" >"$work/stop"
        change_aws s3 cp "$work/stop" "$S3_DEST/run/stop" --only-show-errors || die "cannot write the stop object"
        log "the machine looks for it every 30 seconds, stops the chain with a final checkpoint, saves, records the" \
            "end, takes the group to capacity 0 and powers off; wall.sh status shows when"
    fi
}

# --- cleanup -----------------------------------------------------------------------------------------------------

# The instances of the project that are not gone, as arguments of aws.
live_instances=(ec2 describe-instances --filters Name=tag:Project,Values=taxman
    Name=instance-state-name,Values=pending,running,shutting-down,stopping,stopped
    --query 'Reservations[].Instances[].InstanceId')

# Waits until no instance of the project is left (for the group's machines to be gone), at most ten minutes.
wait_for_no_instances() {
    local tries=0 left
    show_read "${live_instances[@]}" --output text
    while :; do
        left="$("$AWS_CLI" "${live_instances[@]}" --output text)" || die "cannot read the instances"
        [ -z "$left" ] && return 0
        tries=$((tries + 1))
        [ "$tries" -le 40 ] || die "still there after ten minutes: $left"
        printf '  waiting for %s ...\n' "$left" >&2
        sleep 15
    done
}

cleanup() {
    local force=0
    [ "${1:-}" = --force ] && force=1
    need_account
    if [ "$force" != 1 ] && ! has_object ended; then
        die "the run has not ended ($S3_DEST/run/ended is not there): wall.sh stop first, or cleanup --force"
    fi
    local sg tries
    change_aws autoscaling delete-auto-scaling-group --auto-scaling-group-name "$NAME" --force-delete ||
        log "no group, or not deleted"
    [ "$DRY_RUN" = 1 ] || wait_for_no_instances
    change_aws ec2 delete-launch-template --launch-template-name "$NAME" || log "no launch template, or not deleted"
    sg="$(read_aws ec2 describe-security-groups --filters "Name=group-name,Values=$NAME" \
        Name=tag:Project,Values=taxman --query 'SecurityGroups[].GroupId' --output text)"
    if [ -n "$sg" ]; then
        tries=0
        # The network interface of a terminated instance goes a minute or two after it.
        until change_aws ec2 delete-security-group --group-id "$sg"; do
            tries=$((tries + 1))
            [ "$tries" -le 12 ] || die "cannot delete the security group $sg"
            sleep 10
        done
    fi
    change_aws iam delete-role-policy --role-name "$ROLE" --policy-name "$NAME" || log "no policy, or not deleted"

    printf '\nthe end checks (each must print nothing, [] or None):\n'
    check_aws "${live_instances[@]}"
    check_aws ec2 describe-volumes --filters Name=tag:Project,Values=taxman --query 'Volumes[].VolumeId'
    check_aws ec2 describe-spot-instance-requests --filters Name=tag:Project,Values=taxman \
        Name=state,Values=open,active --query 'SpotInstanceRequests[].SpotInstanceRequestId'
    check_aws autoscaling describe-auto-scaling-groups --auto-scaling-group-names "$NAME" \
        --query 'AutoScalingGroups[].AutoScalingGroupName'
    check_aws ec2 describe-launch-templates --filters "Name=launch-template-name,Values=$NAME" \
        --query 'LaunchTemplates[].LaunchTemplateName'
    check_aws ec2 describe-security-groups --filters Name=tag:Project,Values=taxman --query 'SecurityGroups[].GroupId'
    check_aws iam list-role-policies --role-name "$ROLE" --query 'PolicyNames'
    printf '\nthe state of the run stays in %s (results, checkpoints, logs); the bucket and the role %s are kept.\n' \
        "$S3_DEST" "$ROLE"
}

# --- main --------------------------------------------------------------------------------------------------------

# --verbose goes with any command, before it or after (and VERBOSE=1 does without it); so do --recorded N and --host
# USER@HOST, which status takes.
args=()
while [ "$#" -gt 0 ]; do
    case "$1" in
        --verbose) VERBOSE=1 ;;
        --recorded)
            [ "$#" -ge 2 ] || die "--recorded needs a value (see the top of $0)"
            is_whole_number "$2" || die "--recorded: \"$2\" is not a whole number"
            SHOW_RECORDED="$2"
            shift
            ;;
        --host)
            [ "$#" -ge 2 ] || die "--host needs a value (see the top of $0)"
            # USER@HOST, so that ssh cannot take it for an option.
            case "$2" in
                -* | *[!A-Za-z0-9_.:@%+-]* | @* | *@ | *@*@*) die "--host: \"$2\" is not USER@HOST" ;;
                *@*) ;;
                *) die "--host: \"$2\" is not USER@HOST" ;;
            esac
            WALL_HOST="$2"
            shift
            ;;
        *) args+=("$1") ;;
    esac
    shift
done
set -- ${args[@]+"${args[@]}"}
command="${1:-}"
[ "$#" -gt 0 ] && shift
# launch starts a run, so it takes run1 when RUN is not set; status, stop and cleanup take the run in progress (need_run).
case "$command" in
    launch) [ -n "$RUN" ] || set_run run1 ;;
esac
[ -z "$WALL_HOST" ] || [ "$command" = status ] || die "--host is for status: wall.sh status --host USER@HOST"
case "$command" in
    launch) launch "$@" ;;
    status) if [ -n "$WALL_HOST" ]; then status_host; else status; fi ;;
    stop) stop "$@" ;;
    cleanup) cleanup "$@" ;;
    -h | --help | help) usage ;;
    *) die "usage: wall.sh launch --commit SHA --until DATE [options] | status [--recorded N] [--host USER@HOST]" \
        "| stop [--now] | cleanup [--force]   (any command: --verbose)" ;;
esac
