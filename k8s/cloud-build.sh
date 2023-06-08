#!/bin/sh
# first arg : name of image to build
set -e


SCRIPT_DIR="$(cd "$(dirname "$0")" >/dev/null 2>&1 && pwd)"

# second arg : directory to build from (defaults to service name arg)
BUILD_DIR=${2:-$1}

# Change the current working directory to the directory in which to build the image
cd "${SCRIPT_DIR}/../${BUILD_DIR}"

# Check the current working directory
echo "Current working directory is $(pwd)"

# Start the build and get its ID
echo "Starting build..."
BUILD_ID=$(gcloud builds submit --quiet --config "${SCRIPT_DIR}/cloudbuild.yaml" --substitutions=SHORT_SHA=$(git rev-parse --short HEAD),_IMAGE_NAME=$1 --format='value(id)' .)
