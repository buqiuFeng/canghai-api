package com.canghai.api.service;

import com.alibaba.fastjson2.JSON;
import com.canghai.api.common.ApiResult;
import com.canghai.api.common.ErrorCode;
import com.canghai.api.entity.Category;
import com.canghai.api.entity.Environment;
import com.canghai.api.entity.EnvironmentGroup;
import com.canghai.api.entity.EnvironmentVariable;
import com.canghai.api.entity.Project;
import com.canghai.api.entity.ProjectMember;
import com.canghai.api.entity.SavedRequest;
import com.canghai.api.entity.Team;
import com.canghai.api.entity.TeamMember;
import com.canghai.api.entity.User;
import com.canghai.api.mapper.CategoryMapper;
import com.canghai.api.mapper.EnvironmentGroupMapper;
import com.canghai.api.mapper.EnvironmentMapper;
import com.canghai.api.mapper.EnvironmentVariableMapper;
import com.canghai.api.mapper.ProjectMapper;
import com.canghai.api.mapper.ProjectMemberMapper;
import com.canghai.api.mapper.SavedRequestMapper;
import com.canghai.api.mapper.TeamMapper;
import com.canghai.api.mapper.TeamMemberMapper;
import com.canghai.api.model.SyncData;
import com.canghai.api.model.SyncResponse;
import com.baomidou.mybatisplus.core.conditions.query.QueryWrapper;
import com.baomidou.mybatisplus.core.conditions.update.UpdateWrapper;
import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.stereotype.Service;
import org.springframework.transaction.annotation.Transactional;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.function.Consumer;
import java.util.function.Function;
import java.util.function.Predicate;

/**
 * 同步服务 — 客户端数据上报与服务端数据下发。
 * 冲突策略：以 updateTime 最新者为准（last-write-wins）。
 * POST /api/sync/upload → 客户端上报数据，服务端合并后返回全量
 * GET  /api/sync/pull   → 拉取服务端最新数据
 */
@Service
public class SyncService extends BaseService {

    @Autowired
    private CategoryMapper categoryMapper;
    @Autowired
    private SavedRequestMapper savedRequestMapper;
    @Autowired
    private EnvironmentGroupMapper environmentGroupMapper;
    @Autowired
    private EnvironmentMapper environmentMapper;
    @Autowired
    private EnvironmentVariableMapper environmentVariableMapper;
    @Autowired
    private TeamMapper teamMapper;
    @Autowired
    private TeamMemberMapper teamMemberMapper;
    @Autowired
    private ProjectMapper projectMapper;
    @Autowired
    private ProjectMemberMapper projectMemberMapper;
    /** 用于读取数据库时钟生成同步游标（避免 JVM 时区与 DB 时区不一致） */
    @Autowired
    private org.springframework.jdbc.core.JdbcTemplate jdbcTemplate;



    // ==================== 增量同步：时间游标与版本戳 ====================

    /** 游标格式：与 DATETIME(3) 列读取出的字符串格式一致，可直接参与列比较 */
    private static final java.time.format.DateTimeFormatter STAMP_FMT =
            java.time.format.DateTimeFormatter.ofPattern("yyyy-MM-dd HH:mm:ss.SSS");

    /** 标准 lastSyncTime 取值：空/空串视为全量 */
    private static String normalizeCursor(String cursor) {
        return cursor == null ? "" : cursor.trim();
    }

    /**
     * 本次响应的服务端时间游标（客户端应回传作为下次 lastSyncTime）。
     *
     * <p>取数据库的 {@code NOW(3)}，与 {@code server_update_time} 列（由 MySQL 的
     * {@code CURRENT_TIMESTAMP(3)} 维护）出自**同一个会话时钟**（连接串把会话时区钉在
     * Asia/Shanghai），因此增量条件 {@code server_update_time >= lastSyncTime} 天然自洽：
     * 即使时区配置被改错，游标与列值也会一起偏移，而不会互相错位。
     *
     * <p>历史上这里用 {@code UTC_TIMESTAMP(3)}：当时库在 UTC、JVM 在 UTC+8，必须靠
     * 「取数据库时钟」来对齐；现在三端基准统一为北京时间后，{@code NOW(3)} 更稳。
     *
     * <p>同时刻意以「字符串」而不是 Timestamp 读取：Timestamp 会经过 JDBC 驱动的时区换算
     * （受 connectionTimeZone / JVM 默认时区影响），而 {@code DATE_FORMAT} 返回的是
     * 数据库里的字面值，与列值完全同源，免疫驱动换算。
     */
    private String serverTimeNow() {
        try {
            // 输出 DATETIME(3) 对应的 23 字符形式 yyyy-MM-dd HH:mm:ss.SSS
            String s = jdbcTemplate.queryForObject(
                    "SELECT DATE_FORMAT(NOW(3), '%Y-%m-%d %H:%i:%s.%f')", String.class);
            if (s != null && s.length() >= 23) {
                return s.substring(0, 23);
            }
        } catch (Exception ignored) {
            // 取不到 DB 时钟时回退为 JVM 的本地（Asia/Shanghai）时间
        }
        return java.time.LocalDateTime.now(java.time.ZoneId.of("Asia/Shanghai")).format(STAMP_FMT);
    }

    /** 服务端行版本戳：update_time（秒级）+ version（同秒自增），用于冲突判定 */
    private static final class Stamp {
        final String updateTime;
        final int version;

        Stamp(String updateTime, int version) {
            this.updateTime = updateTime == null ? "" : updateTime;
            this.version = version;
        }
    }

    /**
     * 冲突判定：> 0 表示客户端更新（覆盖服务端）。
     * 先比 update_time；同一秒内的并发修改再比 version（由 DB 触发器自增）。
     */
    private static int compareToServer(String clientTime, Integer clientVersion, Stamp server) {
        String ct = clientTime == null ? "" : clientTime;
        int c = ct.compareTo(server.updateTime);
        if (c != 0) return c;
        return (clientVersion == null ? 0 : clientVersion) - server.version;
    }

    /**
     * 上报合并：5 类实体的 upsert 与软删必须整体原子，否则中断会留下半合并状态。
     * timeout 防止大批量合并长事务拖垮连接池。
     */
    @Transactional(rollbackFor = Exception.class, timeout = 120)
    public ApiResult<SyncResponse> upload(SyncData data, User user) {
        // 同步不再按 teamId 作用域：数据按「项目成员归属」收集与合并，团队仅经项目成员关系间接关联。
        // 上传是「写」操作，按项目级角色裁剪：仅可写项目（OWNER/ADMIN/READWRITE，或所在团队角色非只读）
        // 的条目参与合并；只读项目的条目被舍去。响应仍下发可见项目全集（含只读），供客户端拉取展示。
        if (data == null) {
            data = new SyncData();
        }

        List<String> visibleProjectIds = projectMemberService.getVisibleProjectIds(user);
        List<String> writableProjectIds = projectMemberService.getWritableProjectIds(user);

        mergeCategories(writableProjectIds, data.getCategories(), user.getId());
        mergeRequests(writableProjectIds, data.getSavedRequests(), user.getId());
        mergeEnvironmentGroups(writableProjectIds, data.getEnvironmentGroups(), user.getId());
        mergeEnvironments(writableProjectIds, data.getEnvironments(), user.getId());
        mergeEnvironmentVariables(data.getEnvironmentVariables(), user.getId());

        return ApiResult.ok(buildResponse(visibleProjectIds, user, normalizeCursor(data.getLastSyncTime())));
    }

    public ApiResult<SyncResponse> pull(SyncData req, User user) {
        List<String> visibleProjectIds = projectMemberService.getVisibleProjectIds(user);
        String lastSyncTime = req != null ? normalizeCursor(req.getLastSyncTime()) : "";
        return ApiResult.ok(buildResponse(visibleProjectIds, user, lastSyncTime));
    }

    // ==================== 合并逻辑 ====================

    /**
     * 按项目归属过滤后合并：先批量判定「需写入」的集合，再一次性 upsert（Phase 7.3）。
     *
     * <p>改为「先收集、后批量」后，1000 行合并由 2000+ 次逐行 round-trip 降为
     * 每实体 1~5 次批量语句（按 200 行分片）；冲突判定规则本身未变。
     */
    private <T> void mergeByProjectIds(List<T> clientList, List<String> projectIds,
                                       Function<T, String> idFn, Function<T, String> projFn,
                                       Function<T, String> tsFn, Function<T, Integer> verFn,
                                       Predicate<T> delFn,
                                       Function<List<String>, Map<String, Stamp>> loadServerMap,
                                       Consumer<List<T>> batchUpsertFn, Consumer<T> deleteFn) {
        if (clientList == null || clientList.isEmpty() || projectIds == null || projectIds.isEmpty()) return;
        Set<String> allowed = new HashSet<>(projectIds);
        List<T> scoped = clientList.stream().filter(i -> allowed.contains(projFn.apply(i))).toList();
        if (scoped.isEmpty()) return;

        Map<String, Stamp> serverMap = loadServerMap.apply(projectIds);
        List<T> toUpsert = new ArrayList<>();
        for (T item : scoped) {
            String id = idFn.apply(item);
            if (id == null) continue;
            Stamp serverStamp = serverMap.get(id);
            // 服务端不存在，或客户端更新 → 需要写入（insert 与 update 统一走 upsert）
            if (serverStamp == null || compareToServer(tsFn.apply(item), verFn.apply(item), serverStamp) > 0) {
                toUpsert.add(item);
            }
        }
        if (!toUpsert.isEmpty()) {
            batchUpsertFn.accept(toUpsert);
        }
        for (T item : scoped) {
            if (Boolean.TRUE.equals(delFn.test(item))) {
                deleteFn.accept(item);
            }
        }
    }

    /** 全局实体（无项目归属）的合并：语义同 {@link #mergeByProjectIds}，但服务端戳按 id 批量加载。 */
    private <T> void mergeGlobal(List<T> clientList, Function<T, String> idFn, Function<T, String> tsFn,
                                 Function<T, Integer> verFn, Predicate<T> delFn,
                                 Function<List<String>, Map<String, Stamp>> loadServerMap,
                                 Consumer<List<T>> batchUpsertFn, Consumer<T> deleteFn) {
        if (clientList == null || clientList.isEmpty()) return;
        List<String> ids = clientList.stream().map(idFn).filter(java.util.Objects::nonNull).toList();
        if (ids.isEmpty()) return;
        Map<String, Stamp> serverMap = loadServerMap.apply(ids);
        List<T> toUpsert = new ArrayList<>();
        for (T item : clientList) {
            String id = idFn.apply(item);
            if (id == null) continue;
            Stamp serverStamp = serverMap.get(id);
            if (serverStamp == null || compareToServer(tsFn.apply(item), verFn.apply(item), serverStamp) > 0) {
                toUpsert.add(item);
            }
        }
        if (!toUpsert.isEmpty()) {
            batchUpsertFn.accept(toUpsert);
        }
        for (T item : clientList) {
            if (Boolean.TRUE.equals(delFn.test(item))) deleteFn.accept(item);
        }
    }

    private void mergeCategories(List<String> projectIds, List<Category> clientList, String userId) {
        mergeByProjectIds(clientList, projectIds,
                Category::getId, Category::getProjectId, Category::getUpdateTime, Category::getSyncVersion,
                c -> Boolean.TRUE.equals(c.getDeleted()),
                pids -> loadServerStampMap(categoryMapper, "project_id", pids),
                this::batchUpsertCategories, c -> softDelete(categoryMapper, c.getId()));
    }

    private void mergeRequests(List<String> projectIds, List<SavedRequest> clientList, String userId) {
        mergeByProjectIds(clientList, projectIds,
                SavedRequest::getId, SavedRequest::getProjectId, SavedRequest::getUpdateTime, SavedRequest::getSyncVersion,
                r -> Boolean.TRUE.equals(r.getDeleted()),
                pids -> loadServerStampMap(savedRequestMapper, "project_id", pids),
                this::batchUpsertRequests, r -> softDelete(savedRequestMapper, r.getId()));
    }

    private void mergeEnvironmentGroups(List<String> projectIds, List<EnvironmentGroup> clientList, String userId) {
        mergeByProjectIds(clientList, projectIds,
                EnvironmentGroup::getId, EnvironmentGroup::getProjectId, EnvironmentGroup::getUpdateTime, EnvironmentGroup::getSyncVersion,
                g -> Boolean.TRUE.equals(g.getDeleted()),
                pids -> loadServerStampMap(environmentGroupMapper, "project_id", pids),
                this::batchUpsertEnvGroups, g -> softDelete(environmentGroupMapper, g.getId()));
    }

    private void mergeEnvironments(List<String> projectIds, List<Environment> clientList, String userId) {
        mergeByProjectIds(clientList, projectIds,
                Environment::getId, Environment::getProjectId, Environment::getUpdateTime, Environment::getSyncVersion,
                e -> Boolean.TRUE.equals(e.getDeleted()),
                pids -> loadServerStampMap(environmentMapper, "project_id", pids),
                this::batchUpsertEnvs, e -> softDelete(environmentMapper, e.getId()));
    }

    private void mergeEnvironmentVariables(List<EnvironmentVariable> clientList, String userId) {
        mergeGlobal(clientList, EnvironmentVariable::getId, EnvironmentVariable::getUpdateTime,
                EnvironmentVariable::getSyncVersion, v -> Boolean.TRUE.equals(v.getDeleted()),
                ids -> loadServerStampMapById(environmentVariableMapper, ids),
                this::batchUpsertEnvVars, v -> softDelete(environmentVariableMapper, v.getId()));
    }

    /** 批量分片大小：避免单条 SQL 过大触发 max_allowed_packet（路线图 §10 风险应对） */
    private static final int BATCH_CHUNK_SIZE = 200;

    private static <T> void inChunks(List<T> list, Consumer<List<T>> fn) {
        for (int i = 0; i < list.size(); i += BATCH_CHUNK_SIZE) {
            fn.accept(list.subList(i, Math.min(list.size(), i + BATCH_CHUNK_SIZE)));
        }
    }

    /** 加载服务端行的 (update_time, version) 版本戳：按项目列批量过滤 */
    private <E> Map<String, Stamp> loadServerStampMap(com.baomidou.mybatisplus.core.mapper.BaseMapper<E> mapper,
                                                      String col, List<String> values) {
        return toStampMap(mapper.selectList(new QueryWrapper<E>()
                .select("id", "update_time", "sync_version").in(col, values).eq("deleted", 0)));
    }

    /** 加载服务端行的 (update_time, version) 版本戳：按 id 批量过滤 */
    private <E> Map<String, Stamp> loadServerStampMapById(com.baomidou.mybatisplus.core.mapper.BaseMapper<E> mapper,
                                                          List<String> ids) {
        return toStampMap(mapper.selectList(new QueryWrapper<E>()
                .select("id", "update_time", "sync_version").in("id", ids).eq("deleted", 0)));
    }

    private <E> Map<String, Stamp> toStampMap(List<E> rows) {
        Map<String, Stamp> map = new HashMap<>();
        try {
            for (E r : rows) {
                java.lang.reflect.Method gid = r.getClass().getMethod("getId");
                java.lang.reflect.Method gut = r.getClass().getMethod("getUpdateTime");
                java.lang.reflect.Method gv = r.getClass().getMethod("getSyncVersion");
                Object id = gid.invoke(r);
                Object ut = gut.invoke(r);
                Object ver = gv.invoke(r);
                if (id == null) continue;
                int version = ver instanceof Number n ? n.intValue() : 0;
                map.put(id.toString(), new Stamp(ut == null ? "" : ut.toString(), version));
            }
        } catch (Exception ignored) {
        }
        return map;
    }

    // ==================== 批量 upsert（Phase 7.3） ====================
    // 逐行 insert/update 已合并为「先归一化默认值、再按 200 行分片批量 upsert」，
    // 单条 SQL 同时覆盖「服务端不存在（插入）」与「客户端更新（覆盖）」两种情形。

    private void batchUpsertCategories(List<Category> list) {
        for (Category c : list) {
            c.setProjectId(c.getProjectId() != null ? c.getProjectId() : "");
            c.setName(c.getName() != null ? c.getName() : "");
            c.setSortOrder(c.getSortOrder() != null ? c.getSortOrder() : 0);
            c.setExpanded(c.getExpanded() != null && c.getExpanded());
            c.setCreateTime(c.getCreateTime() != null ? c.getCreateTime() : now());
            c.setUpdateTime(c.getUpdateTime() != null ? c.getUpdateTime() : now());
            c.setDeleted(c.getDeleted() != null && c.getDeleted());
        }
        inChunks(list, categoryMapper::batchUpsert);
    }

    private void batchUpsertRequests(List<SavedRequest> list) {
        for (SavedRequest r : list) {
            r.setProjectId(r.getProjectId() != null ? r.getProjectId() : "");
            r.setName(r.getName() != null ? r.getName() : "");
            r.setMethod(r.getMethod() != null ? r.getMethod() : "GET");
            r.setUrl(r.getUrl() != null ? r.getUrl() : "");
            r.setBodyType(r.getBodyType() != null ? r.getBodyType() : "none");
            r.setBody(r.getBody() != null ? r.getBody() : "");
            r.setPreScript(r.getPreScript() != null ? r.getPreScript() : "");
            r.setPostScript(r.getPostScript() != null ? r.getPostScript() : "");
            r.setSortOrder(r.getSortOrder() != null ? r.getSortOrder() : 0);
            r.setCreateTime(r.getCreateTime() != null ? r.getCreateTime() : now());
            r.setUpdateTime(r.getUpdateTime() != null ? r.getUpdateTime() : now());
            r.setDeleted(r.getDeleted() != null && r.getDeleted());
        }
        inChunks(list, savedRequestMapper::batchUpsert);
    }

    private void batchUpsertEnvGroups(List<EnvironmentGroup> list) {
        for (EnvironmentGroup g : list) {
            g.setProjectId(g.getProjectId() != null ? g.getProjectId() : "");
            g.setName(g.getName() != null ? g.getName() : "");
            g.setSortOrder(g.getSortOrder() != null ? g.getSortOrder() : 0);
            g.setExpanded(g.getExpanded() != null && g.getExpanded());
            g.setCreateTime(g.getCreateTime() != null ? g.getCreateTime() : now());
            g.setUpdateTime(g.getUpdateTime() != null ? g.getUpdateTime() : now());
            g.setDeleted(g.getDeleted() != null && g.getDeleted());
        }
        inChunks(list, environmentGroupMapper::batchUpsert);
    }

    private void batchUpsertEnvs(List<Environment> list) {
        for (Environment e : list) {
            e.setProjectId(e.getProjectId() != null ? e.getProjectId() : "");
            e.setName(e.getName() != null ? e.getName() : "");
            e.setIsActive(e.getIsActive() != null && e.getIsActive());
            e.setSortOrder(e.getSortOrder() != null ? e.getSortOrder() : 0);
            e.setCreateTime(e.getCreateTime() != null ? e.getCreateTime() : now());
            e.setUpdateTime(e.getUpdateTime() != null ? e.getUpdateTime() : now());
            e.setDeleted(e.getDeleted() != null && e.getDeleted());
        }
        inChunks(list, environmentMapper::batchUpsert);
    }

    private void batchUpsertEnvVars(List<EnvironmentVariable> list) {
        for (EnvironmentVariable v : list) {
            v.setVarKey(v.getVarKey() != null ? v.getVarKey() : "");
            v.setValue(v.getValue() != null ? v.getValue() : "");
            v.setEnabled(v.getEnabled() != null && v.getEnabled());
            v.setSortOrder(v.getSortOrder() != null ? v.getSortOrder() : 0);
            v.setCreateTime(v.getCreateTime() != null ? v.getCreateTime() : now());
            v.setUpdateTime(v.getUpdateTime() != null ? v.getUpdateTime() : now());
            v.setDeleted(v.getDeleted() != null && v.getDeleted());
        }
        inChunks(list, environmentVariableMapper::batchUpsert);
    }

    private static String jsonStr(Object o) {
        return o == null ? null : JSON.toJSONString(o);
    }

    // ==================== 构建响应 ====================

    /**
     * 构建同步响应。
     *
     * @param lastSyncTime 客户端游标（服务端时间）。非空表示增量下发：
     *                     5 类同步实体只返回 server_update_time >= lastSyncTime 的行。
     *                     项目/团队/成员数据量小且未纳入游标，始终全量下发。
     *                     <p>注意：删除墓碑（deletedIds）**全量与增量都会下发**，
     *                     否则全量下发时只回传 deleted = 0 的行、墓碑为空，
     *                     客户端本地缓存的已删除行将永远无法清除。
     */
    private SyncResponse buildResponse(List<String> visibleProjectIds, User user, String lastSyncTime) {
        SyncResponse resp = new SyncResponse();
        resp.setSyncAt(System.currentTimeMillis());
        String serverTime = serverTimeNow();
        resp.setServerTime(serverTime);

        boolean incremental = !lastSyncTime.isEmpty();
        // 游标超前保护：客户端升级前可能保存了本地时钟生成的游标（比 DB 时钟快数小时），
        // 这类游标会让增量条件恒不成立、客户端再也拉不到变更，故退化为全量。
        if (incremental && lastSyncTime.compareTo(serverTime) > 0) {
            incremental = false;
        }
        resp.setIncremental(incremental);

        // 墓碑必须在「无可见项目」早返回之前算：
        // 用户唯一/最后的项目被删除时 visibleProjectIds 恰好为空，
        // 若放在早返回之后，客户端将永远收不到该项目的删除通知。
        resp.setDeletedIds(loadDeletedIds(visibleProjectIds, incremental ? lastSyncTime : null, user));

        if (visibleProjectIds.isEmpty()) return resp;

        // 结构类数据（项目/团队/成员）：无游标列，始终全量，保证权限与归属最新
        resp.setProjects(loadProjects(visibleProjectIds));
        resp.setProjectMembers(loadProjectMembers(visibleProjectIds));
        Set<String> teamIds = collectTeamIds(visibleProjectIds, user);
        resp.setTeams(loadTeams(teamIds));
        resp.setTeamMembers(loadTeamMembers(teamIds));

        // 内容类数据（5 张同步表）：增量下发
        resp.setCategories(incremental ? loadCategoriesSince(visibleProjectIds, lastSyncTime) : loadCategories(visibleProjectIds));
        resp.setRequests(incremental ? loadRequestsSince(visibleProjectIds, lastSyncTime) : loadRequests(visibleProjectIds));
        resp.setEnvironmentGroups(incremental ? loadEnvironmentGroupsSince(visibleProjectIds, lastSyncTime) : loadEnvironmentGroups(visibleProjectIds));
        resp.setEnvironments(incremental ? loadEnvironmentsSince(visibleProjectIds, lastSyncTime) : loadEnvironments(visibleProjectIds));
        resp.setEnvironmentVariables(incremental ? loadEnvironmentVariablesSince(visibleProjectIds, lastSyncTime) : loadEnvironmentVariables(visibleProjectIds));

        return resp;
    }

    /**
     * 删除墓碑：服务端已删除、需要客户端清理本地副本的实体 id。
     *
     * <p><b>为什么要下发墓碑</b>：下发只返回 {@code deleted = 0} 的行，客户端合并是
     * 「按 id upsert」，不会主动删除本地多出来的行；若不下发墓碑，服务端删掉的数据
     * 在客户端会永久残留（表现为「服务器删了、客户端还在」）。
     *
     * <p><b>为什么全量也要下发</b>：全量下发同样只回传存活行，若此时墓碑为空，
     * 客户端升级/清库后第一次全量同步就再也清不掉历史残留。因此 {@code since == null}
     * （全量）时下发范围内的全部已删除 id，增量时按游标过滤，两者语义一致。
     *
     * <p>范围：
     * <ul>
     *   <li>5 张同步表：以「可见项目」为范围，增量时按 {@code server_update_time >= since} 过滤；</li>
     *   <li>项目 / 项目成员：项目删除会连同成员记录一并软删，此时 {@code visibleProjectIds}
     *       已不含该项目，故范围改用「曾经可见的项目」；这两张表没有 {@code server_update_time}
     *       列，无法按游标过滤，因此每次同步都全量下发（数据量小，且客户端清理是幂等的）。</li>
     * </ul>
     */
    private List<String> loadDeletedIds(List<String> projectIds, String since, User user) {
        List<String> ids = new ArrayList<>();
        // since 为空 => 全量：不加时间条件；非空 => 增量：按游标过滤区间
        boolean filtered = since != null && !since.isBlank();

        if (projectIds != null && !projectIds.isEmpty()) {
            ids.addAll(selectIds(categoryMapper.selectObjs(new QueryWrapper<Category>()
                    .select("id").in("project_id", projectIds).eq("deleted", 1)
                    .ge(filtered, "server_update_time", since))));
            ids.addAll(selectIds(savedRequestMapper.selectObjs(new QueryWrapper<SavedRequest>()
                    .select("id").in("project_id", projectIds).eq("deleted", 1)
                    .ge(filtered, "server_update_time", since))));
            ids.addAll(selectIds(environmentGroupMapper.selectObjs(new QueryWrapper<EnvironmentGroup>()
                    .select("id").in("project_id", projectIds).eq("deleted", 1)
                    .ge(filtered, "server_update_time", since))));
            ids.addAll(selectIds(environmentMapper.selectObjs(new QueryWrapper<Environment>()
                    .select("id").in("project_id", projectIds).eq("deleted", 1)
                    .ge(filtered, "server_update_time", since))));
            List<Environment> envs = environmentMapper.selectList(new QueryWrapper<Environment>()
                    .in("project_id", projectIds).select("id"));
            if (!envs.isEmpty()) {
                List<String> envIds = new ArrayList<>();
                for (Environment e : envs) envIds.add(e.getId());
                ids.addAll(selectIds(environmentVariableMapper.selectObjs(new QueryWrapper<EnvironmentVariable>()
                        .select("id").in("environment_id", envIds).eq("deleted", 1)
                        .ge(filtered, "server_update_time", since))));
            }
        }

        // 项目 / 项目成员墓碑：范围取「曾经可见」，否则项目删除后无法被客户端感知
        if (user != null) {
            List<String> formerProjectIds = projectMemberService.getFormerlyVisibleProjectIds(user);
            if (!formerProjectIds.isEmpty()) {
                ids.addAll(selectIds(projectMapper.selectObjs(new QueryWrapper<Project>()
                        .select("id").in("id", formerProjectIds).eq("deleted", 1))));
                ids.addAll(selectIds(projectMemberMapper.selectObjs(new QueryWrapper<ProjectMember>()
                        .select("id").in("project_id", formerProjectIds).eq("deleted", 1))));
            }
        }
        return ids;
    }

    private static List<String> selectIds(List<Object> rows) {
        List<String> out = new ArrayList<>();
        if (rows == null) return out;
        for (Object r : rows) if (r != null) out.add(r.toString());
        return out;
    }

    private List<Project> loadProjects(List<String> ids) {
        return projectMapper.selectList(new QueryWrapper<Project>()
                .in("id", ids).eq("deleted", 0).orderByAsc("create_time"));
    }

    // ==================== 下发查询 ====================
    //
    // 注意：以下内容实体（5 张同步表 + 项目成员）**刻意都不过滤 deleted = 0**，
    // 软删行会随列表一起下发（携带 deleted = true），由客户端识别为「删除信号」
    // 并清理本地副本。
    //
    // 为什么不能只靠 deletedIds 墓碑：
    //   墓碑在增量模式下受游标窗口（server_update_time >= lastSyncTime）限制，
    //   一旦客户端的游标越过删除时刻（例如升级前全量同步不返墓碑），该删除就永远
    //   不会再被通知，本地脏数据永久残留。让软删行随列表下发后，客户端每次都能
    //   在完整列表里看到「这一行已被删除」，不再依赖游标窗口。
    //
    // 而项目 / 团队 / 团队成员仍只下发存活行：前者本地无 deleted 列（靠墓碑物理删除），
    // 后两者的删除不参与同步。

    private List<Category> loadCategories(List<String> ids) {
        return categoryMapper.selectList(new QueryWrapper<Category>()
                .in("project_id", ids).orderByAsc("sort_order"));
    }

    private List<Category> loadCategoriesSince(List<String> ids, String since) {
        return categoryMapper.selectList(new QueryWrapper<Category>()
                .in("project_id", ids).ge("server_update_time", since).orderByAsc("sort_order"));
    }

    private List<SavedRequest> loadRequests(List<String> ids) {
        return savedRequestMapper.selectList(new QueryWrapper<SavedRequest>()
                .in("project_id", ids).orderByAsc("sort_order").orderByAsc("name"));
    }

    private List<SavedRequest> loadRequestsSince(List<String> ids, String since) {
        return savedRequestMapper.selectList(new QueryWrapper<SavedRequest>()
                .in("project_id", ids).ge("server_update_time", since).orderByAsc("sort_order").orderByAsc("name"));
    }

    private List<EnvironmentGroup> loadEnvironmentGroups(List<String> ids) {
        return environmentGroupMapper.selectList(new QueryWrapper<EnvironmentGroup>()
                .in("project_id", ids).orderByAsc("sort_order"));
    }

    private List<EnvironmentGroup> loadEnvironmentGroupsSince(List<String> ids, String since) {
        return environmentGroupMapper.selectList(new QueryWrapper<EnvironmentGroup>()
                .in("project_id", ids).ge("server_update_time", since).orderByAsc("sort_order"));
    }

    private List<Environment> loadEnvironments(List<String> ids) {
        return environmentMapper.selectList(new QueryWrapper<Environment>()
                .in("project_id", ids).orderByAsc("create_time"));
    }

    private List<Environment> loadEnvironmentsSince(List<String> ids, String since) {
        return environmentMapper.selectList(new QueryWrapper<Environment>()
                .in("project_id", ids).ge("server_update_time", since).orderByAsc("create_time"));
    }

    private List<ProjectMember> loadProjectMembers(List<String> ids) {
        return projectMemberMapper.selectList(new QueryWrapper<ProjectMember>()
                .in("project_id", ids));
    }

    private Set<String> collectTeamIds(List<String> visibleProjectIds, User user) {
        Set<String> teamIds = new LinkedHashSet<>();
        List<ProjectMember> pm = projectMemberMapper.selectList(new QueryWrapper<ProjectMember>()
                .in("project_id", visibleProjectIds).eq("deleted", 0).eq("member_type", "team").select("member_id"));
        for (ProjectMember r : pm) if (r.getMemberId() != null) teamIds.add(r.getMemberId());
        List<TeamMember> tm = teamMemberMapper.selectList(new QueryWrapper<TeamMember>()
                .eq("user_id", user.getId()).eq("deleted", 0).select("DISTINCT team_id"));
        for (TeamMember r : tm) if (r.getTeamId() != null) teamIds.add(r.getTeamId());
        return teamIds;
    }

    private List<Team> loadTeams(Set<String> teamIds) {
        if (teamIds.isEmpty()) return new ArrayList<>();
        return teamMapper.selectList(new QueryWrapper<Team>().in("id", teamIds).eq("deleted", 0));
    }

    private List<TeamMember> loadTeamMembers(Set<String> teamIds) {
        if (teamIds.isEmpty()) return new ArrayList<>();
        return teamMemberMapper.selectList(new QueryWrapper<TeamMember>().in("team_id", teamIds).eq("deleted", 0));
    }

    private List<EnvironmentVariable> loadEnvironmentVariables(List<String> projectIds) {
        List<String> envIds = visibleEnvironmentIds(projectIds);
        if (envIds.isEmpty()) return new ArrayList<>();
        return environmentVariableMapper.selectList(new QueryWrapper<EnvironmentVariable>()
                .in("environment_id", envIds).orderByAsc("sort_order"));
    }

    private List<EnvironmentVariable> loadEnvironmentVariablesSince(List<String> projectIds, String since) {
        List<String> envIds = visibleEnvironmentIds(projectIds);
        if (envIds.isEmpty()) return new ArrayList<>();
        return environmentVariableMapper.selectList(new QueryWrapper<EnvironmentVariable>()
                .in("environment_id", envIds).ge("server_update_time", since).orderByAsc("sort_order"));
    }

    /** 可见项目下的环境 id（含已删除环境，避免其变量变成孤儿） */
    private List<String> visibleEnvironmentIds(List<String> projectIds) {
        List<Environment> envs = environmentMapper.selectList(new QueryWrapper<Environment>()
                .in("project_id", projectIds).select("id"));
        List<String> envIds = new ArrayList<>();
        for (Environment e : envs) if (e.getId() != null) envIds.add(e.getId());
        return envIds;
    }
}
