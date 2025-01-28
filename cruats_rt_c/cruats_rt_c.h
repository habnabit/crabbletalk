/*
 * © 2022 <_@habnab.it>
 *
 * SPDX-License-Identifier: MPL-2.0
 */

#include <stdarg.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>


typedef struct cruats_at_addr {
  unsigned short s_net;
  unsigned short s_node;
} cruats_at_addr;

typedef struct cruats_sockaddr_at {
  short sat_len;
  short sat_family;
  short sat_port;
  short sat_type;
  struct cruats_at_addr sat_addr;
  char sat_zero[8];
} cruats_sockaddr_at;

int cruats_ddp_close(int socket);

int cruats_ddp_open(struct cruats_sockaddr_at *addr, struct cruats_sockaddr_at *bridge);

ssize_t cruats_ddp_recvfrom(int socket,
                            void *buf,
                            size_t len,
                            int flags,
                            struct cruats_sockaddr_at *addr,
                            size_t *addrlen);

ssize_t cruats_ddp_sendto(int socket,
                          const void *buf,
                          size_t len,
                          int flags,
                          const struct cruats_sockaddr_at *addr,
                          size_t addrlen);
