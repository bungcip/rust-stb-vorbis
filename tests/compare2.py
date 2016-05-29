#!/usr/bin/python

import hashlib
import subprocess
import sys
import os
import time

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

oggs = [
#    'single-code-2bits', 'single-code-nonsparse', 'single-code-ordered', 'single-code-sparse',
#    '6ch-all-page-types', '6ch-long-first-packet', '6ch-moving-sine', '6ch-moving-sine-floor0',
#    'bitrate-123', 'bitrate-456-0', 'bitrate-456-789',
    # 'empty-page',
    # 'noise-6ch', 'noise-stereo',
    # 'sample-rate-max',
    # 'sketch008-floor0', 
    # 'square', 'square-interleaved', 'square-stereo', 
    'stereo_short', 'mono', #~~ cannot be distributed, this is a just song from my playlist...

    #'thingy-floor0', ~~ error 4

    
    # rust-stb-vorbis
    # 'thingy', 'sketch008', 'sketch039', ~~ very slow
    
    # also crash in original C stb-vorbis
    # no need to fix this....
        
    #'6-mode-bits', '6-mode-bits-multipage',
    #'large-pages',
    #'long-short',
    #'zero-length',
    #'partial-granule-position',
    #'square-with-junk', 
    #'bad-continued-packet-flag', 
]

# run stb_vorbis
#print("run stb_vorbic C...")
#for o in oggs:
#    print("  run {}".format(o))
#    subprocess.call(["vorvis-sample.exe", "1", "ogg/{}.ogg".format(o), "c_output/[decode_filename]_{}.out".format(o)])
#    subprocess.call(["vorvis-sample.exe", "5", "ogg/{}.ogg".format(o), "c_output/[decode_frame_pushdata]_{}.out".format(o)])

binaries = [
#   "decode_filename",
    "decode_frame_pushdata",
]

# compile rust port
print("compile stb_vorbis rust example...")
for bin in binaries:
    result = subprocess.call(["cargo", "build", "--example", bin])
    if result != 0:
        sys.exit()


# test output file size
# test output file hash
print("check output file size & hash")

for bin in binaries:
    executable = "../target/debug/examples/{}.exe".format(bin)
    prefix_output = "[{}]".format(bin)

    print("TESTING {}".format(bin))

    for i in oggs:
        filename = "{}_{}.out".format(prefix_output, i)
        c_name = os.path.join('c_output', filename)
        rust_name = os.path.join('rust_output', filename)

        try:
            input = "ogg/{}.ogg".format(i)
            start_time = time.time()
            return_value = subprocess.call([executable, input, rust_name])
            if return_value != 0:
                print("stoped due to error...")
                sys.exit()
            end_time = time.time()
        except:
            print("error happened")
            sys.exit()


        c_size = os.path.getsize(c_name)
        rust_size = os.path.getsize(rust_name)
        if c_size != rust_size:
            print("  [WRONG] size of rust different with original stb_vorbis! filename: {}, size: {}".format(filename, rust_size))
            sys.exit()

        c_md5 = md5(c_name)
        rust_md5 = md5(rust_name)
        if c_md5 != rust_md5:
            print("  [WRONG] rust output have wrong hash! filename: {}, size: {}".format(filename, rust_size))
            sys.exit()

        print("  [OK] in {:.3f} seconds".format(end_time - start_time))

