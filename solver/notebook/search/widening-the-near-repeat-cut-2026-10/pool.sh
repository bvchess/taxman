#!/bin/bash
# pool.sh JOBS: runs each line of the file JOBS as a command of one thread, so that at most 8 run at a time, or 4 while
# a lab study runs (the last start line of ~/taxman-lab/ctl/on-event.log has no exit after it).  It counts every
# process of this clone's programs and builds (those of other pools too), and when a study starts it stops (SIGSTOP)
# those past the fourth until the study exits; CPU limits are unaffected.  Logs to JOBS.log.
jobs_file=$1
log=$jobs_file.log
study_running() {
    local last
    last=$(grep -E " (start|exit) event=" /home/brian/taxman-lab/ctl/on-event.log | tail -1)
    [[ $last == *" start event="* ]]
}
running() {
    # The processes that take a thread: this clone's programs, and the compiler of its builds.
    local clone=research-widen/target/release
    pgrep -u "$(id -u)" -f "$clone/examples/|$clone/deps/|rustc .*research-widen" | sort -n
}
throttle() {
    local limit=$1 i=0
    for p in $(running); do
        if [ $i -lt "$limit" ]; then kill -CONT "$p" 2> /dev/null; else kill -STOP "$p" 2> /dev/null; fi
        i=$((i + 1))
    done
}
limit_now() {
    if study_running; then echo 4; else echo 8; fi
}
while IFS= read -r line; do
    [ -z "$line" ] && continue
    while true; do
        limit=$(limit_now)
        throttle "$limit"
        [ "$(running | wc -l)" -lt "$limit" ] && break
        sleep 20
    done
    echo "$(date -u +%FT%TZ) start $line" >> "$log"
    setsid bash -c "$line; echo \"\$(date -u +%FT%TZ) done \$? $line\" >> $log" &
    sleep 5
done < "$jobs_file"
while [ "$(running | wc -l)" -gt 0 ]; do
    throttle "$(limit_now)"
    sleep 20
done
echo "$(date -u +%FT%TZ) all done" >> "$log"
