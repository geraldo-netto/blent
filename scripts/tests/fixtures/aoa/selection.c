#include "accessory.h"
#include <stdio.h>
int main(int argc,char **argv) {
    if(argc!=4) return 9;
    accessory_selection selected=accessory_parse_selection(argv[1],argv[2],argv[3]);
    printf("%u %u %u\n",selected.address,selected.vendor,selected.product);
    return 0;
}
