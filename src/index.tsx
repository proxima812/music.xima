/* @refresh reload */
import { render } from 'solid-js/web'

import '@/styles/index.css'

import { App } from '@/app/App'

const root = document.getElementById('root')

if (root === null) {
  throw new Error('index.tsx: в index.html нет элемента #root')
}

render(() => <App />, root)
