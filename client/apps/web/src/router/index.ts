import { createRouter, createWebHashHistory } from 'vue-router'
import ProjectSelectView from '@/views/ProjectSelectView.vue'
import ApiDebuggerView from '@/views/ApiDebuggerView.vue'

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: '/',
      name: 'project-select',
      component: ProjectSelectView,
    },
    {
      path: '/project/:projectId',
      name: 'api-debugger',
      component: ApiDebuggerView,
      props: true,
    },
    // 兜底：未知路径回到项目选择
    { path: '/:pathMatch(.*)*', redirect: '/' },
  ],
  scrollBehavior() {
    return { top: 0 }
  },
})

export default router
