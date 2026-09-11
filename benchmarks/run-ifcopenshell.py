import sys, time, os
import ifcopenshell
def rss_mb():
    for l in open("/proc/self/status"):
        if l.startswith("VmRSS:"): return int(l.split()[1])/1024
    return 0
path = sys.argv[1]
mb = os.path.getsize(path)/1048576
base = rss_mb()
t = time.time()
f = ifcopenshell.open(path)
n = len(list(f))
secs = time.time()-t
print(f"impl=ifcopenshell file={mb:.0f}MB entities={n} secs={secs:.3f} rss_MB={rss_mb()-base:.0f}")
