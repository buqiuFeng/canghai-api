# ===== 沧海 API 调试工具 后端 Dockerfile（Spring Boot）=====
# 构建后端可执行 fat jar（Java 21），运行时用 JRE 21。
FROM maven:3.9-eclipse-temurin-21 AS build
WORKDIR /app
COPY spring-boot-server/pom.xml .
COPY spring-boot-server/src ./src
RUN mvn -q -DskipTests package

FROM eclipse-temurin:21-jre
WORKDIR /app
COPY --from=build /app/target/spring-boot-server.jar /app/app.jar
COPY spring-boot-server/src/main/resources /app/resources
EXPOSE 8092
ENV JAVA_OPTS="-Xmx512m"
# 默认读取 classpath:keys/private.pem（已在 .gitignore 中，不会进入仓库）；
# 生产部署请通过环境变量 CANGHAI_JWT_PRIVATE_KEY 指向自有密钥文件后重建镜像。
ENTRYPOINT ["sh", "-c", "java $JAVA_OPTS -jar /app/app.jar"]
