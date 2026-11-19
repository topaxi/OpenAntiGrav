// LibAtrac9 as a test oracle: decodes raw ATRAC9 superframes to s16le.
// Used by `atrac9_against_libatrac9_sweep` in crates/music/tests/atrac9_ground_truth.rs;
// docs/formats/atrac9.md has the build line. LibAtrac9 is MIT (Alex Barney).
// usage: atrac9-oracle <config word, 8 hex digits> <frames file> <superframe bytes> <out.pcm>
#include <stdio.h>
#include <stdlib.h>
#include "libatrac9.h"
int main(int argc, char **argv) {
  unsigned char cfg[4]; unsigned v = strtoul(argv[1], 0, 16);
  cfg[0]=v>>24; cfg[1]=v>>16; cfg[2]=v>>8; cfg[3]=v;
  FILE *f = fopen(argv[2], "rb"); fseek(f,0,SEEK_END); long n=ftell(f); fseek(f,0,SEEK_SET);
  unsigned char *buf = malloc(n); fread(buf,1,n,f); fclose(f);
  int sf = atoi(argv[3]);
  void *h = Atrac9GetHandle(); int e = Atrac9InitDecoder(h, cfg); if (e) { fprintf(stderr,"init %d\n",e); return 2; }
  Atrac9CodecInfo info; Atrac9GetCodecInfo(h,&info);
  short *pcm = malloc(sizeof(short)*info.channels*info.frameSamples);
  FILE *o = fopen(argv[4],"wb");
  for (long at=0; at+sf<=n; at+=sf) {
    long q = at;
    for (int fr=0; fr<info.framesInSuperframe; fr++) {
      int used=0; e = Atrac9Decode(h, buf+q, pcm, &used);
      if (e) { fprintf(stderr,"superframe %ld frame %d err %d\n", at/sf, fr, e); return 3; }
      fwrite(pcm, sizeof(short), info.channels*info.frameSamples, o);
      q += used;
    }
  }
  fclose(o); return 0;
}
