#!/bin/bash
# Landscape calibration and map (approved 2026-10-17, $2): standalone bounds 2176-3440 against the chain,
# 64 standalone one-thread searches of 30 minutes at most, then bounds 3441-8191; to S3, then power off.
shutdown -h +240
export AWS_DEFAULT_REGION=us-east-1
D=s3://taxman-013764204840-us-east-1/landscape-20261017
dnf install -y -q git tmux
sudo -u ec2-user bash -lc 'cd ~ && git clone -q https://github.com/bvchess/taxman.git && cd taxman && git checkout -q 6f57399 && SKIP_TESTS=1 rust/tools/cloud/setup.sh > ~/setup.log 2>&1'
cat > /home/ec2-user/land.sh <<'EOF'
#!/bin/bash
set -u
D=s3://taxman-013764204840-us-east-1/landscape-20261017
O=~/land; mkdir -p $O/sa; cd ~/taxman/rust
( while sleep 300; do aws s3 sync --quiet $O $D/out; done ) &
echo "start $(date -u +%FT%TZ)" >> $O/log
target/release/examples/wall landscape 2176-3440 --threads 64 --known results/strict.jsonl --out $O/cal.jsonl > $O/cal.out 2>&1
echo "cal done $(date -u +%FT%TZ)" >> $O/log
echo 2196 2203 2265 2289 2299 2312 2345 2351 2388 2392 2416 2423 2440 2478 2485 2528 2530 2531 2535 2555 2560 2565 2601 2632 2640 2655 2696 2700 2704 2712 2713 2750 2784 2799 2800 2849 2925 2976 2997 3000 3003 3037 3061 3064 3105 3112 3115 3125 3129 3156 3169 3171 3204 3209 3221 3259 3318 3331 3335 3368 3384 3393 3404 3440 | tr ' ' '\n' | xargs -P 64 -I{} sh -c 'target/release/taxman solve {} --threads 1 --time-limit 1800 --progress 0 --out '$O'/sa/{}.jsonl > '$O'/sa/{}.out 2>&1'
echo "standalone done $(date -u +%FT%TZ)" >> $O/log
target/release/examples/wall landscape 3441-8191 --threads 64 --out $O/map.jsonl > $O/map.out 2>&1
echo "map done $(date -u +%FT%TZ)" >> $O/log
aws s3 sync --quiet $O $D/out
sudo shutdown -h now
EOF
sed -i "s/SAMPLE/'""'/" /dev/null
chown ec2-user /home/ec2-user/land.sh
sudo -u ec2-user tmux new-session -d -s land 'bash ~/land.sh'
