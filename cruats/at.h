/*
 * © 2022 <_@habnab.it>
 *
 * SPDX-License-Identifier: MPL-2.0
 */

struct at_addr {
    unsigned short s_net, s_node;
};

struct sockaddr_at {
    short sat_len, sat_family, sat_port, sat_type;
    struct at_addr sat_addr;
    char sat_zero[8];
};
