#!/usr/bin/env python3

"""
Internal script to help build `#include<>`s for the base patches. You probably
don't need this unless you're writing a base patch for another fork.
"""

import os

for path, dirs, files in os.walk("."):
    if path.startswith("./tools"):
        continue

    for f in files:
        if f.endswith(".h"):
            print(f"#include <{path}/{f}>")
