package com.canghai.api.dto.resp;

import com.canghai.api.entity.User;
import lombok.Data;

@Data
public class LoginResponse {
    private String token;
    private User user;
    private Long expiresAt;
}
