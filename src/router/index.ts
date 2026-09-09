import { createRouter, createWebHistory } from 'vue-router'
import Home from '../views/Home.vue'
import MachineCode from '../views/MachineCode.vue'
import Thermal from '../views/Thermal.vue'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', name: 'home', component: Home },
    { path: '/machine-code', name: 'machine-code', component: MachineCode },
    { path: '/thermal', name: 'thermal', component: Thermal },
  ],
})

export default router
