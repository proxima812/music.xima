import { A } from '@solidjs/router'
import { Library, ListMusic, House, Search, type LucideProps } from 'lucide-solid'
import { For, type Component } from 'solid-js'
import { Dynamic } from 'solid-js/web'

type NavItem = {
  href: string
  label: string
  icon: Component<LucideProps>
  /** Точное совпадение пути — нужно только корню, иначе он подсвечен везде. */
  exact: boolean
}

const NAV_ITEMS: readonly NavItem[] = [
  { href: '/', label: 'Главная', icon: House, exact: true },
  { href: '/library', label: 'Библиотека', icon: Library, exact: false },
  { href: '/search', label: 'Поиск', icon: Search, exact: false },
  { href: '/playlists', label: 'Плейлисты', icon: ListMusic, exact: false },
]

/** Нижняя навигация: 4 вкладки, активная — токеном accent, с учётом safe-area. */
export function BottomNav() {
  return (
    <nav
      aria-label="Основная навигация"
      class="depth-bar safe-bottom w-full border-t border-border"
    >
      <ul class="flex h-nav w-full items-stretch">
        <For each={NAV_ITEMS}>
          {(item) => (
            <li class="flex min-w-0 flex-1">
              <A
                href={item.href}
                end={item.exact}
                class="flex min-h-11 w-full flex-col items-center justify-center gap-1 px-1 no-highlight transition-colors duration-150"
                activeClass="text-accent"
                inactiveClass="text-muted"
              >
                <Dynamic component={item.icon} size={22} aria-hidden="true" />
                <span class="w-full truncate text-center text-[11px] leading-none font-medium">
                  {item.label}
                </span>
              </A>
            </li>
          )}
        </For>
      </ul>
    </nav>
  )
}
