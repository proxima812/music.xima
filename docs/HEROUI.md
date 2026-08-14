# HeroUI v3 в music.xima — справочник

Извлечено из `@heroui/styles@3.2.4`. **Не выдумывать классы** — если нужного нет
в списке ниже, значит его нет; пишем свой Tailwind поверх токенов темы.

## Как подключено

`src/styles/index.css` делает ровно один импорт:

```css
@import '@heroui/styles/css';
```

Он уже включает `@import "tailwindcss"`, `tw-animate-css`, base-слой, все
компонентные стили, дефолтную тему и утилиты. **Второй раз tailwind импортировать
нельзя.**

Точечные импорты (если позже захотим урезать бандл) выглядят так:

```css
@import '@heroui/styles/base';
@import '@heroui/styles/components/button.css';
@import '@heroui/styles/themes/default';
```

`@heroui/react` **не устанавливаем** — это React-библиотека. Поведение берём из
`@kobalte/core`, а HeroUI-классы вешаем на его примитивы.

## Тема

Дефолтная тема — светлая. Тёмная включается классом `.dark` **или** атрибутом
`[data-theme="dark"]` на корне. В `index.html` уже стоит `class="dark" data-theme="dark"`.
Есть ещё `[data-vibrant-palette="true"]` — более насыщенная палитра.

### Цветовые токены (Tailwind-классы работают: `bg-surface`, `text-muted`, `border-border`)

```
background  background-secondary  background-tertiary  background-inverse
foreground  muted  link  focus  border  border-secondary  border-tertiary
surface  surface-hover  surface-secondary  surface-tertiary  surface-foreground
overlay  overlay-foreground  backdrop  separator  segment  scrollbar
field  field-border  field-border-focus  field-border-hover  field-foreground
field-placeholder  field-hover  field-focus

default   default-foreground   default-hover   default-soft   default-soft-foreground
accent    accent-foreground    accent-hover    accent-soft    accent-soft-foreground
success   success-foreground   success-hover   success-soft   success-soft-foreground
warning   warning-foreground   warning-hover   warning-soft   warning-soft-foreground
danger    danger-foreground    danger-hover    danger-soft    danger-soft-foreground
```

### Прочие токены

```
--radius-xs/sm/md/lg/xl/2xl/3xl/4xl   --radius-field
--shadow-surface  --shadow-overlay  --shadow-field
--ease-smooth  --ease-out-expo  --ease-out-fluid  --ease-in-out-cubic  (+ полный набор ease-*)
--animate-skeleton  --animate-spin-fast  --animate-caret-blink
--disabled-opacity  --cursor-interactive  --cursor-disabled
```

### Утилиты HeroUI

```
focus-ring  focus-field-ring  invalid-field-ring  no-highlight
status-focused  status-focused-field  status-disabled  status-pending  status-invalid-field
scrollbar  scrollbar-thin  scrollbar-none  scrollbar-default
```

## BEM-классы компонентов

Формат: `.block`, `.block__element`, `.block--modifier`. Состояния читаются с
data-атрибутов (`data-hovered`, `data-pressed`, `data-focus-visible`, `data-selected`,
`data-disabled`, `data-pending`) — Kobalte как раз их и выставляет.

Компоненты, которые реально нужны в этом приложении:

| Блок | Классы |
| --- | --- |
| **button** | `.button` + `--primary --secondary --tertiary --ghost --outline --danger --danger-soft --icon-only --full-width --sm --md --lg` |
| **toggle-button** | `.toggle-button` + `--default --ghost --icon-only --sm --md --lg` |
| **button-group** | `.button-group` + `--horizontal --vertical --full-width`, `.button-group__separator` |
| **card** | `.card` + `--default --secondary --tertiary --transparent`, `.card__header __title __description __content __footer` |
| **surface** | `.surface` + `--default --secondary --tertiary --transparent` |
| **avatar** | `.avatar` + `--sm --md --lg --soft`, `.avatar__image __fallback` (+ `__fallback--accent/danger/success/warning/default`) |
| **chip** | `.chip` + `--primary --secondary --tertiary --default --accent --success --warning --danger --sm --md --lg`, `.chip__label` |
| **tag / tag-group** | `.tag` + `--default --surface --sm --md --lg`, `.tag__remove-button`, `.tag-group`, `.tag-group__list` |
| **slider** | `.slider` (структура собирается Kobalte, стилизуется через токены) |
| **progress-bar** | `.progress-bar` + `--default --accent --success --warning --danger --sm --md --lg` |
| **progress-circle** | `.progress-circle` + те же модификаторы |
| **spinner** | `.spinner` + `--current --accent --success --warning --danger --sm --md --lg --xl` |
| **skeleton** | `.skeleton` + `--pulse --shimmer --none` |
| **tabs** | `.tabs` + `--secondary`, `.tabs__list __list-container __tab __panel __indicator __separator` |
| **list-box** | `.list-box`, `.list-box-item` (+ `--default --danger`), `.list-box-item__indicator`, `.list-box-section` |
| **menu / dropdown** | `.menu`, `.menu-item` (+ `--default --danger`), `.menu-item__indicator`, `.menu-section`, `.dropdown`, `.dropdown__trigger __menu __popover` |
| **modal** | `.modal__backdrop` (+ `--blur --opaque --transparent`), `.modal__container __dialog __header __heading __body __footer __close-trigger __trigger __icon`, размеры `.modal__dialog--xs/sm/md/lg/cover/full` |
| **drawer** | `.drawer__backdrop __dialog __content __header __heading __body __footer __handle __close-trigger`, `.drawer__content--top/right/bottom/left` |
| **alert-dialog** | `.alert-dialog__*` (аналогично modal) |
| **popover** | `.popover`, `.popover__trigger __dialog __heading` |
| **tooltip** | `.tooltip`, `.tooltip__trigger` |
| **toast** | `.toast-region` (+ позиции), `.toast` (+ `--success --warning --danger --accent`), `.toast__title __description __content __action __close-button __indicator` |
| **input / textfield** | `.textfield`, `.input` (+ `--primary --secondary --full-width`), `.label` (+ `--required --invalid --disabled`), `.description`, `.error-message`, `.field-error` |
| **search-field** | `.search-field` (+ `--primary --secondary --full-width`), `.search-field__group __input __search-icon __clear-button` |
| **input-group** | `.input-group` (+ `--primary --secondary --full-width`), `.input-group__input __prefix __suffix` |
| **select** | `.select` (+ `--full-width`), `.select__trigger __value __indicator __popover` |
| **switch** | `.switch` + `--sm --md --lg`, `.switch__control __thumb __label __content` |
| **checkbox / radio** | `.checkbox`, `.checkbox__control __indicator __content`; `.radio`, `.radio__control __indicator __content` |
| **separator** | `.separator` + `--horizontal --vertical --default --secondary --tertiary`, `.separator__line __content __container` |
| **scroll-shadow** | `.scroll-shadow` + `--vertical --horizontal --fade --hide-scrollbar` |
| **empty-state** | `.empty-state` |
| **typography** | `.typography` + `--h1..--h6 --body --body-sm --body-xs --code --truncate --weight-normal/medium/semibold/bold --color-default --color-muted --align-start/center/end/justify` |
| **kbd** | `.kbd` + `--light`, `.kbd__content __abbr` |
| **accordion / disclosure** | `.accordion` (+ `--surface`), `.accordion__item __heading __trigger __panel __indicator __body`; `.disclosure`, `.disclosure__trigger __content __body __indicator` |
| **toolbar** | `.toolbar` + `--horizontal --vertical --attached` |
| **badge** | `.badge` + варианты/позиции, `.badge-anchor`, `.badge__label` |
| **link** | `.link` |
| **table** | `.table-root`, `.table__header __body __row __cell __column __content ...` |

Полный список — `node_modules/@heroui/styles/dist/components/*.css`.

## Правила использования

1. Сначала ищем готовый класс/токен HeroUI, только потом пишем свой Tailwind.
2. Никаких hex/rgb/oklch в компонентах. Цвет — только через токены темы.
3. Новый общий визуальный элемент → `src/shared/ui/`, а не копипаста по фичам.
4. Kobalte даёт поведение и `data-*`, HeroUI — внешний вид. Не тащить в Kobalte-компонент
   собственные `:hover`-стили, если есть `data-hovered`.
5. Плотность интерфейса мобильная: тач-таргет ≥ 44px, `.button--icon-only` для иконок,
   `--sm` только там, где элемент не тапается.
