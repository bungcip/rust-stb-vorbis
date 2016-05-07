#!/usr/bin/python

import hashlib
import subprocess
import sys
import os

# compare output file between original stb_vorbis and rust port
#
#
# c_output       ~~ location of stb_vorbis output
# rust_output    ~~ location of rust port output
# ogg            ~~ ogg sample

def md5(fname):
    hash_md5 = hashlib.md5()
    with open(fname, "rb") as f:
        for chunk in iter(lambda: f.read(4096), b""):
            hash_md5.update(chunk)
    return hash_md5.hexdigest()

# run stb_vorbis
#print("run stb_vorbic C...")
#subprocess.call(["vorvis-sample.exe", "1", "ogg/018.ogg", "c_output/[decode_filename]_018.out"])
#subprocess.call(["vorvis-sample.exe", "1", "ogg/044.ogg", "c_output/[decode_filename]_044.out"])
#subprocess.call(["vorvis-sample.exe", "1", "ogg/048.ogg", "c_output/[decode_filename]_048.out"])

# run rust port
print("run stb_vorbis Rust...")
result = subprocess.call(["cargo", "build", "--example", "decode_filename"])
if result == 0:
    try:
        executable = "../target/debug/examples/decode_filename.exe"
        subprocess.call([executable, "ogg/018.ogg", "rust_output/[decode_filename]_018.out"])
        subprocess.call([executable, "ogg/044.ogg", "rust_output/[decode_filename]_044.out"])
        subprocess.call([executable, "ogg/048.ogg", "rust_output/[decode_filename]_048.out"])
    except ex:
        print("error happened")
        sys.exit()
else:
    sys.exit()


# test output file size
# test output file hash
print("check output file size & hash")
ouputs = [
    '[decode_filename]_018.out','[decode_filename]_044.out', '[decode_filename]_048.out'
]

for i in ouputs:
    c_name = os.path.join('c_output', i)
    rust_name = os.path.join('rust_output', i)

    c_size = os.path.getsize(c_name)
    rust_size = os.path.getsize(rust_name)
    if c_size != rust_size:
        print("[WRONG] size of rust different with original stb_vorbis! filename: {}, size: {}".format(i, rust_size))
        sys.exit()

    c_md5 = md5(c_name)
    rust_md5 = md5(rust_name)
    if c_md5 != rust_md5:
        print("[WRONG] rust output have wrong hash! filename: {}, size: {}".format(i, rust_size))
        sys.exit()

    print("[OK]")

