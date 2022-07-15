#! /bin/sh
#
#  set these:
searchfor=3815b31c5b87e4e4b395cf85bed8760c5802140c

startpoints="master"  # branch names or HEAD or whatever
# you can use rev-list limiters too, e.g., origin/master..master

git rev-list $startpoints |
    while read commithash; do
        if git ls-tree -d -r --full-tree $commithash | grep $searchfor; then
            echo " -- found at $commithash"
        fi
    done
