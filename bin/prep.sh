#!/bin/bash

# bring the system up to date
sudo yum -y update
sudo yum clean metadata

# get java
ssh -i ../aws/bvc2.pem ec2-user@$EC2NAME
wget https://download.java.net/java/GA/jdk18.0.2.1/db379da656dc47308e138f21b33976fa/1/GPL/openjdk-18.0.2.1_linux-x64_bin.tar.gz
tar -xvf openjdk-18.0.2.1_linux-x64_bin.tar.gz
rm openjdk-18.0.2.1_linux-x64_bin.tar.gz
sudo mv jdk-18.0.2.1 /opt
sudo tee /etc/profile.d/jdk.sh <<EOF
export JAVA_HOME=/opt/jdk-18.0.2.1
export PATH=\$PATH:\$JAVA_HOME/bin
alias c=clear
EOF
source /etc/profile.d/jdk.sh


# grow the disk
sudo growpart /dev/nvme0n1 1
sudo xfs_growfs -d /
df -h .

# do an initial run
bin/taxman 456 456 456 520
