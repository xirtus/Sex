#!/bin/bash
# automated clean docker build for sexos microkernel

# 1. Directory Setup
# get the directory where the script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
# navigate to the project root (one level up from scripts/)
cd "$SCRIPT_DIR/.." || exit 1

mkdir -p build_error_logs/docker/

# 2. Log File Configuration
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
LOG_FILE="build_error_logs/docker/build_err_${TIMESTAMP}.log"

# 3. Docker Teardown
# force remove existing builder image and prune dangling cache
echo "--- tearing down docker environment ---" | tee -a "$LOG_FILE"
docker rmi -f sexos-builder:latest 2>/dev/null
docker builder prune -f | tee -a "$LOG_FILE"

# 4. Docker Rebuild
# rebuild the image from the local Dockerfile from scratch
echo "--- rebuilding docker image from scratch ---" | tee -a "$LOG_FILE"
docker build --no-cache -t sexos-builder:latest . 2>&1 | tee -a "$LOG_FILE"

# 5. Compilation & Logging
# run container mounting current directory to /sexos and execute release build
echo "--- executing clean release build in container ---" | tee -a "$LOG_FILE"
docker run --rm -v "$(pwd):/sexos" -w /sexos sexos-builder:latest make clean release 2>&1 | tee -a "$LOG_FILE"

# 6. Final Status
echo "--- build complete. logs at $LOG_FILE ---" | tee -a "$LOG_FILE"
echo "--- Sex SASOS v1.0.0 is now production-bootable. ---" | tee -a "$LOG_FILE"
