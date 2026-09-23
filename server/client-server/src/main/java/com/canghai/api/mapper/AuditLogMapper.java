package com.canghai.api.mapper;

import com.baomidou.mybatisplus.core.mapper.BaseMapper;
import com.canghai.api.entity.AuditLog;
import org.apache.ibatis.annotations.Mapper;

@Mapper
public interface AuditLogMapper extends BaseMapper<AuditLog> {
}
