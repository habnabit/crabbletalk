
struct at_addr {
    unsigned short	s_net;
    unsigned char	s_node;
};

struct sockaddr_at {
    unsigned char sat_len;
    short sat_family;
    unsigned char sat_port;
    struct at_addr sat_addr;
    char sat_zero[ 8 ];
};
