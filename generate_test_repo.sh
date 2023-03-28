#!/bin/bash

git init

echo "inital" > file-a.txt
git add file-a.txt
git commit -m "first commit"

echo "2" >> file-a.txt
echo "first" > file-b.txt
git add file-*.txt
git commit -m "second commit"

git checkout -b b-branch
echo "second" >> file-b.txt
git add file-b.txt
git commit -m "branch commit"

git checkout main
echo "3" >> file-a.txt
git add file-a.txt
git commit -m "modifying main"

git checkout b-branch
