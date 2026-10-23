#!/bin/bash
# queue.sh FILE P: runs the shell lines of FILE, P at a time; its pid in FILE.pid
cd /home/brian/taxman-lab/work/research-stepcost
echo $$ > $1.pid
xargs -P $2 -I{} -d "\n" sh -c "{}" < $1
rm -f $1.pid
