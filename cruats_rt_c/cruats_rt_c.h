#include <stdarg.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>


typedef struct cruats_at_addr {
  unsigned short s_net;
  unsigned char s_node;
} cruats_at_addr;

typedef struct cruats_sockaddr_at {
  unsigned char sat_len;
  short sat_family;
  unsigned char sat_port;
  struct cruats_at_addr sat_addr;
  char sat_zero[8];
} cruats_sockaddr_at;

int cruats_ddp_close(int socket);

int cruats_ddp_open(struct cruats_sockaddr_at *addr, struct cruats_sockaddr_at *bridge);

ssize_t cruats_ddp_recvfrom(int socket,
                            void *buf,
                            size_t len,
                            int flags,
                            void *addr,
                            unsigned int *addrlen);

ssize_t cruats_ddp_sendto(int socket,
                          const void *buf,
                          size_t len,
                          int flags,
                          const void *addr,
                          unsigned int addrlen);
