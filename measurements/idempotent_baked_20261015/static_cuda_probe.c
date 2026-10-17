#define _GNU_SOURCE
#include <dlfcn.h>
#include <link.h>
#include <stdio.h>
#include <pthread.h>
static int (*init_cuda)(unsigned);
static void *invoke(void *unused){(void)unused;return (void*)(long)init_cuda(0);}
int main(void){
  void *h=dlmopen(LM_ID_NEWLM,"/usr/lib/wsl/lib/libcuda.so.1",RTLD_NOW|RTLD_LOCAL);
  if(!h){fprintf(stderr,"dlmopen: %s\n",dlerror());return 1;}
  init_cuda=dlsym(h,"cuInit");
  int (*create)(pthread_t*,const pthread_attr_t*,void*(*)(void*),void*)=dlsym(h,"pthread_create");
  int (*join)(pthread_t,void**)=dlsym(h,"pthread_join");
  if(!init_cuda||!create||!join){fprintf(stderr,"dlsym: %s\n",dlerror());return 1;}
#ifdef CUDA_DRIVER_THREAD
  pthread_t thread;
  int status=create(&thread,0,invoke,0);
  if(status){fprintf(stderr,"driver pthread_create=%d\n",status);return 1;}
  void *returned=0;
  status=join(thread,&returned);
  if(status){fprintf(stderr,"driver pthread_join=%d\n",status);return 1;}
  int result=(int)(long)returned;
  printf("driver-thread cuInit=%d\n",result);return result!=0;
#else
  int result=init_cuda(0);printf("cuInit=%d\n",result);return result!=0;
#endif
}
