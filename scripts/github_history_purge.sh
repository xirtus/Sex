#!/bin/bash
# SexOS SASOS - GitHub History Eradication
set -euo pipefail

# 1. Create a replacement map
echo "tranny==>tuxedo" > access_map.txt
echo "Tranny==>Tuxedo" >> access_map.txt
echo "sextranny==>tuxedo" >> access_map.txt
echo "Sextranny==>Tuxedo" >> access_map.txt
echo "sextran==>tuxedo" >> access_map.txt

echo "--> PHASE 1: Rewriting all commit history and messages..."
# This is the nuclear option. It replaces strings in files AND commit logs.
git filter-repo --replace-text access_map.txt --force

echo "--> PHASE 2: Force-pushing the new sanitized history to GitHub..."
# Note: This will overwrite EVERYTHING on GitHub with the new clean history.
git push origin --force --all
git push origin --force --tags

echo "--> SUCCESS: History purged. Old commit hashes are now dead/404."
rm access_map.txt
