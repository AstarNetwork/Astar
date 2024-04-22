#!/bin/bash
################################################################################
################################################################################
##
## DESCRIPTION
##  This script will update the year in all license field on top of files.
##
## USAGE
##
## 1. Specifies the the new year & old year to be updated: NEW_YEAR & OLD_YEAR.
## 3. Uses grep to find all files containing the old SPDX-License-Identifier.
## The script iterates over each file found and uses sed to replace the old SPDX-License-Identifier with the updated one, including the new year.
##
################################################################################
################################################################################

# Specify the old year
OLD_YEAR="2023"

# Specify the new year
NEW_YEAR="2024"

DIRECTORIES="../bin ../chain-extensions ../pallets ../precompiles ../primitives ../rpc-tests ../runtime ../scripts ../tests ../third-party ../vendor"

# Find all source files with the old year in the copyright notice in the parent directory and its subdirectories
FILES=$(find $DIRECTORIES -type f -exec grep -l -F -e "Copyright (C) 2019-$OLD_YEAR" {} +)

# Iterate over each file and update the year
for FILE in $FILES; do
    sed -i '' -e "s/Copyright (C) 2019-$OLD_YEAR/Copyright (C) 2019-$NEW_YEAR/g" "$FILE"
    echo "Updated $FILE"
done
