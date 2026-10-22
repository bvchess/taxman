#!/bin/bash
# queue.sh FILE P: runs the shell lines of FILE, P at a time
cd /home/brian/taxman-lab/work/research-cb
xargs -P $2 -I{} -d "\n" sh -c "{}" < $1
