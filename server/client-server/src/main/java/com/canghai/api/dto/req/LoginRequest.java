package com.canghai.api.dto.req;

import lombok.Data;

@Data
public class LoginRequest {
    private String username;
    private String password;
    private String nickname;
    private String email;
}
