#define _POSIX_C_SOURCE 200809L
#include "gpu.h"
#include "checked_arguments.h"
#include <X11/extensions/Xrender.h>
#include <assert.h>
#include <stdio.h>
#include <string.h>
static Cursor shape(Display *d,unsigned size,unsigned color){Window root=DefaultRootWindow(d);Pixmap p=XCreatePixmap(d,root,size,size,32);GC gc=XCreateGC(d,p,0,NULL);XSetForeground(d,gc,color);XFillRectangle(d,p,gc,0,0,size,size);XFreeGC(d,gc);Picture pic=XRenderCreatePicture(d,p,XRenderFindStandardFormat(d,PictStandardARGB32),0,NULL);Cursor c=XRenderCreateCursor(d,pic,0,0);XRenderFreePicture(d,pic);XFreePixmap(d,p);return c;}
int main(int argc,char **argv){assert(argc==3);assert(getenv("BLENT_C_REVIEW_ISOLATED"));int size=benchmark_number(argv[1],8,128),churn=benchmark_number(argv[2],0,1);gpu_capture c={.width=256,.height=256};c.display=XOpenDisplay(NULL);assert(c.display);c.root=DefaultRootWindow(c.display);c.pixmap=XCreatePixmap(c.display,c.root,256,256,24);Cursor shapes[]={shape(c.display,size,0xffaa0011),shape(c.display,size,0xff00aa11)};XDefineCursor(c.display,c.root,shapes[0]);XWarpPointer(c.display,None,c.root,0,0,0,0,64,64);XSync(c.display,False);
 for(int i=0;i<16;i++)gpu_cursor(&c);XSync(c.display,False);unsigned long requests=NextRequest(c.display);uint64_t cpu=gpu_now_ns();
 for(int i=0;i<1024;i++){if(churn){XDefineCursor(c.display,c.root,shapes[i%2]);XSync(c.display,False);}gpu_cursor(&c);}XSync(c.display,False);uint64_t elapsed=gpu_now_ns()-cpu;
 printf("{\"size\":%d,\"churn\":%d,\"ns\":%.3f,\"requests\":%lu}\n",size,churn,(double)elapsed/1024,NextRequest(c.display)-requests);gpu_cursor_close(&c);XFreeCursor(c.display,shapes[0]);XFreeCursor(c.display,shapes[1]);XFreePixmap(c.display,c.pixmap);XCloseDisplay(c.display);return 0;}
