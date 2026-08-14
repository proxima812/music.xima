/**
 * Glow-градиенты вместо отсутствующей обложки.
 *
 * Палитра взята с https://color.xima.work/collection/glow/ — все 39 градиентов
 * коллекции, `background-image` слово в слово с сайта. Это данные, а не
 * вычисление: подбирать «похожие» цвета кодом бессмысленно, коллекция уже
 * собрана и сбалансирована.
 *
 * Выбор градиента детерминирован семенем (обычно `album·artist`): один и тот же
 * альбом всегда получает одну и ту же картинку — в списке, в мини-плеере и на
 * весь экран, и после переустановки тоже.
 */

export type GlowGradient = {
  /** Слаг с сайта — чтобы можно было найти оригинал. */
  name: string
  /** Значение `background-image`. */
  image: string
}

export const GLOW_GRADIENTS: readonly GlowGradient[] = [
  {
    name: 'glow-aurora-cyan',
    image:
      'radial-gradient(45% 60% at 18% 18%, rgba(34, 211, 238, 0.62) 0%, rgba(34, 211, 238, 0) 100%), radial-gradient(48% 62% at 82% 16%, rgba(59, 130, 246, 0.54) 0%, rgba(59, 130, 246, 0) 100%), radial-gradient(70% 95% at 52% 84%, rgba(56, 189, 248, 0.46) 0%, rgba(56, 189, 248, 0) 100%), linear-gradient(145deg, #ecfeff 0%, #e0f2fe 55%, #dbeafe 100%)',
  },
  {
    name: 'glow-violet-pulse',
    image:
      'radial-gradient(46% 64% at 16% 18%, rgba(167, 139, 250, 0.64) 0%, rgba(167, 139, 250, 0) 100%), radial-gradient(52% 68% at 84% 14%, rgba(192, 132, 252, 0.56) 0%, rgba(192, 132, 252, 0) 100%), radial-gradient(72% 96% at 50% 86%, rgba(216, 180, 254, 0.44) 0%, rgba(216, 180, 254, 0) 100%), linear-gradient(150deg, #faf5ff 0%, #f3e8ff 55%, #ede9fe 100%)',
  },
  {
    name: 'glow-ember-rose',
    image:
      'radial-gradient(44% 60% at 14% 14%, rgba(251, 113, 133, 0.62) 0%, rgba(251, 113, 133, 0) 100%), radial-gradient(50% 66% at 84% 18%, rgba(244, 114, 182, 0.54) 0%, rgba(244, 114, 182, 0) 100%), radial-gradient(74% 98% at 50% 86%, rgba(253, 164, 175, 0.46) 0%, rgba(253, 164, 175, 0) 100%), linear-gradient(150deg, #fff1f2 0%, #ffe4e6 52%, #fdf2f8 100%)',
  },
  {
    name: 'glow-lime-spark',
    image:
      'radial-gradient(46% 62% at 14% 16%, rgba(163, 230, 53, 0.66) 0%, rgba(163, 230, 53, 0) 100%), radial-gradient(50% 66% at 84% 14%, rgba(74, 222, 128, 0.54) 0%, rgba(74, 222, 128, 0) 100%), radial-gradient(74% 96% at 50% 88%, rgba(190, 242, 100, 0.46) 0%, rgba(190, 242, 100, 0) 100%), linear-gradient(150deg, #f7fee7 0%, #ecfccb 52%, #dcfce7 100%)',
  },
  {
    name: 'glow-cobalt-beam',
    image:
      'radial-gradient(45% 62% at 16% 18%, rgba(96, 165, 250, 0.66) 0%, rgba(96, 165, 250, 0) 100%), radial-gradient(50% 68% at 82% 14%, rgba(59, 130, 246, 0.58) 0%, rgba(59, 130, 246, 0) 100%), radial-gradient(72% 96% at 52% 86%, rgba(125, 211, 252, 0.44) 0%, rgba(125, 211, 252, 0) 100%), linear-gradient(148deg, #eff6ff 0%, #dbeafe 50%, #e0f2fe 100%)',
  },
  {
    name: 'glow-sunset-lava',
    image:
      'radial-gradient(46% 64% at 14% 14%, rgba(251, 191, 36, 0.66) 0%, rgba(251, 191, 36, 0) 100%), radial-gradient(52% 66% at 84% 16%, rgba(249, 115, 22, 0.58) 0%, rgba(249, 115, 22, 0) 100%), radial-gradient(74% 96% at 50% 86%, rgba(253, 186, 116, 0.44) 0%, rgba(253, 186, 116, 0) 100%), linear-gradient(150deg, #fffbeb 0%, #fef3c7 50%, #ffedd5 100%)',
  },
  {
    name: 'glow-mint-plasma',
    image:
      'radial-gradient(44% 60% at 16% 12%, rgba(45, 212, 191, 0.64) 0%, rgba(45, 212, 191, 0) 100%), radial-gradient(50% 66% at 84% 14%, rgba(20, 184, 166, 0.56) 0%, rgba(20, 184, 166, 0) 100%), radial-gradient(72% 94% at 50% 86%, rgba(94, 234, 212, 0.44) 0%, rgba(94, 234, 212, 0) 100%), linear-gradient(150deg, #ecfeff 0%, #ccfbf1 52%, #d1fae5 100%)',
  },
  {
    name: 'glow-orchid-flash',
    image:
      'radial-gradient(46% 62% at 16% 14%, rgba(232, 121, 249, 0.64) 0%, rgba(232, 121, 249, 0) 100%), radial-gradient(50% 66% at 82% 14%, rgba(217, 70, 239, 0.56) 0%, rgba(217, 70, 239, 0) 100%), radial-gradient(72% 96% at 50% 88%, rgba(240, 171, 252, 0.44) 0%, rgba(240, 171, 252, 0) 100%), linear-gradient(148deg, #fdf4ff 0%, #fae8ff 52%, #f5d0fe 100%)',
  },
  {
    name: 'glow-glacier-neon',
    image:
      'radial-gradient(46% 62% at 14% 12%, rgba(103, 232, 249, 0.64) 0%, rgba(103, 232, 249, 0) 100%), radial-gradient(50% 66% at 84% 14%, rgba(34, 211, 238, 0.56) 0%, rgba(34, 211, 238, 0) 100%), radial-gradient(72% 96% at 50% 88%, rgba(125, 211, 252, 0.44) 0%, rgba(125, 211, 252, 0) 100%), linear-gradient(150deg, #ecfeff 0%, #cffafe 52%, #e0f2fe 100%)',
  },
  {
    name: 'glow-fuchsia-nova',
    image:
      'radial-gradient(44% 60% at 14% 12%, rgba(244, 114, 182, 0.66) 0%, rgba(244, 114, 182, 0) 100%), radial-gradient(50% 66% at 84% 16%, rgba(236, 72, 153, 0.56) 0%, rgba(236, 72, 153, 0) 100%), radial-gradient(74% 96% at 50% 88%, rgba(249, 168, 212, 0.44) 0%, rgba(249, 168, 212, 0) 100%), linear-gradient(150deg, #fdf2f8 0%, #fce7f3 52%, #fbcfe8 100%)',
  },
  {
    name: 'glow-pluto-dusk',
    image:
      'radial-gradient(44% 60% at 14% 10%, rgba(167, 139, 250, 0.62) 0%, rgba(167, 139, 250, 0) 100%), radial-gradient(52% 68% at 86% 16%, rgba(96, 165, 250, 0.46) 0%, rgba(96, 165, 250, 0) 100%), radial-gradient(76% 98% at 50% 88%, rgba(216, 180, 254, 0.42) 0%, rgba(216, 180, 254, 0) 100%), linear-gradient(150deg, #f5f3ff 0%, #e9d5ff 50%, #dbeafe 100%)',
  },
  {
    name: 'glow-pluto-ice',
    image:
      'radial-gradient(46% 62% at 15% 12%, rgba(125, 211, 252, 0.62) 0%, rgba(125, 211, 252, 0) 100%), radial-gradient(48% 64% at 84% 14%, rgba(196, 181, 253, 0.5) 0%, rgba(196, 181, 253, 0) 100%), radial-gradient(74% 96% at 52% 88%, rgba(103, 232, 249, 0.42) 0%, rgba(103, 232, 249, 0) 100%), linear-gradient(148deg, #ecfeff 0%, #e0f2fe 52%, #eef2ff 100%)',
  },
  {
    name: 'glow-nebula-orchid',
    image:
      'radial-gradient(44% 62% at 16% 10%, rgba(232, 121, 249, 0.66) 0%, rgba(232, 121, 249, 0) 100%), radial-gradient(50% 66% at 84% 12%, rgba(168, 85, 247, 0.56) 0%, rgba(168, 85, 247, 0) 100%), radial-gradient(72% 96% at 50% 90%, rgba(244, 114, 182, 0.4) 0%, rgba(244, 114, 182, 0) 100%), linear-gradient(150deg, #fdf4ff 0%, #fae8ff 52%, #fce7f3 100%)',
  },
  {
    name: 'glow-nebula-citrine',
    image:
      'radial-gradient(46% 62% at 14% 10%, rgba(250, 204, 21, 0.68) 0%, rgba(250, 204, 21, 0) 100%), radial-gradient(50% 66% at 84% 14%, rgba(251, 146, 60, 0.56) 0%, rgba(251, 146, 60, 0) 100%), radial-gradient(74% 98% at 52% 90%, rgba(253, 224, 71, 0.42) 0%, rgba(253, 224, 71, 0) 100%), linear-gradient(150deg, #fefce8 0%, #fef3c7 50%, #ffedd5 100%)',
  },
  {
    name: 'glow-orbit-azure',
    image:
      'radial-gradient(44% 60% at 14% 12%, rgba(56, 189, 248, 0.68) 0%, rgba(56, 189, 248, 0) 100%), radial-gradient(50% 68% at 86% 16%, rgba(59, 130, 246, 0.58) 0%, rgba(59, 130, 246, 0) 100%), radial-gradient(74% 96% at 50% 88%, rgba(14, 165, 233, 0.42) 0%, rgba(14, 165, 233, 0) 100%), linear-gradient(150deg, #ecfeff 0%, #dbeafe 52%, #bfdbfe 100%)',
  },
  {
    name: 'glow-orbit-rose',
    image:
      'radial-gradient(44% 60% at 14% 12%, rgba(251, 113, 133, 0.66) 0%, rgba(251, 113, 133, 0) 100%), radial-gradient(52% 66% at 84% 14%, rgba(244, 114, 182, 0.56) 0%, rgba(244, 114, 182, 0) 100%), radial-gradient(74% 98% at 50% 88%, rgba(254, 205, 211, 0.42) 0%, rgba(254, 205, 211, 0) 100%), linear-gradient(150deg, #fff1f2 0%, #fce7f3 52%, #fbcfe8 100%)',
  },
  {
    name: 'glow-cosmos-velvet',
    image:
      'radial-gradient(48% 66% at 12% 10%, rgba(129, 140, 248, 0.58) 0%, rgba(129, 140, 248, 0) 100%), radial-gradient(52% 68% at 86% 12%, rgba(192, 132, 252, 0.5) 0%, rgba(192, 132, 252, 0) 100%), radial-gradient(78% 100% at 50% 90%, rgba(14, 165, 233, 0.3) 0%, rgba(14, 165, 233, 0) 100%), linear-gradient(150deg, #1e1b4b 0%, #312e81 54%, #1f2937 100%)',
  },
  {
    name: 'glow-asteroid-amber',
    image:
      'radial-gradient(46% 62% at 16% 12%, rgba(251, 191, 36, 0.68) 0%, rgba(251, 191, 36, 0) 100%), radial-gradient(50% 66% at 84% 16%, rgba(249, 115, 22, 0.6) 0%, rgba(249, 115, 22, 0) 100%), radial-gradient(72% 96% at 50% 88%, rgba(253, 186, 116, 0.42) 0%, rgba(253, 186, 116, 0) 100%), linear-gradient(148deg, #fffbeb 0%, #fef3c7 52%, #fed7aa 100%)',
  },
  {
    name: 'glow-lunar-mint',
    image:
      'radial-gradient(46% 62% at 14% 12%, rgba(94, 234, 212, 0.62) 0%, rgba(94, 234, 212, 0) 100%), radial-gradient(50% 66% at 84% 14%, rgba(45, 212, 191, 0.56) 0%, rgba(45, 212, 191, 0) 100%), radial-gradient(74% 96% at 50% 88%, rgba(209, 250, 229, 0.42) 0%, rgba(209, 250, 229, 0) 100%), linear-gradient(150deg, #ecfeff 0%, #ccfbf1 52%, #dcfce7 100%)',
  },
  {
    name: 'glow-solar-lilac',
    image:
      'radial-gradient(46% 62% at 14% 10%, rgba(250, 204, 21, 0.62) 0%, rgba(250, 204, 21, 0) 100%), radial-gradient(52% 66% at 86% 16%, rgba(216, 180, 254, 0.56) 0%, rgba(216, 180, 254, 0) 100%), radial-gradient(76% 98% at 50% 88%, rgba(192, 132, 252, 0.4) 0%, rgba(192, 132, 252, 0) 100%), linear-gradient(150deg, #fefce8 0%, #f5f3ff 54%, #fae8ff 100%)',
  },
  {
    name: 'glow-galactic-berry',
    image:
      'radial-gradient(44% 60% at 14% 12%, rgba(244, 114, 182, 0.66) 0%, rgba(244, 114, 182, 0) 100%), radial-gradient(52% 66% at 84% 14%, rgba(168, 85, 247, 0.58) 0%, rgba(168, 85, 247, 0) 100%), radial-gradient(74% 96% at 50% 88%, rgba(236, 72, 153, 0.42) 0%, rgba(236, 72, 153, 0) 100%), linear-gradient(148deg, #fdf2f8 0%, #fae8ff 52%, #f5d0fe 100%)',
  },
  {
    name: 'glow-deep-space-teal',
    image:
      'radial-gradient(46% 64% at 14% 12%, rgba(34, 211, 238, 0.52) 0%, rgba(34, 211, 238, 0) 100%), radial-gradient(52% 68% at 86% 14%, rgba(45, 212, 191, 0.46) 0%, rgba(45, 212, 191, 0) 100%), radial-gradient(76% 100% at 50% 90%, rgba(56, 189, 248, 0.28) 0%, rgba(56, 189, 248, 0) 100%), linear-gradient(150deg, #082f49 0%, #0f766e 52%, #0f172a 100%)',
  },
  {
    name: 'glow-comet-cyan',
    image:
      'radial-gradient(44% 60% at 12% 10%, rgba(103, 232, 249, 0.66) 0%, rgba(103, 232, 249, 0) 100%), radial-gradient(50% 66% at 84% 12%, rgba(34, 211, 238, 0.56) 0%, rgba(34, 211, 238, 0) 100%), radial-gradient(72% 94% at 50% 88%, rgba(56, 189, 248, 0.42) 0%, rgba(56, 189, 248, 0) 100%), linear-gradient(150deg, #ecfeff 0%, #cffafe 50%, #bae6fd 100%)',
  },
  {
    name: 'glow-equinox-violet',
    image:
      'radial-gradient(46% 62% at 14% 12%, rgba(196, 181, 253, 0.66) 0%, rgba(196, 181, 253, 0) 100%), radial-gradient(52% 68% at 86% 14%, rgba(167, 139, 250, 0.56) 0%, rgba(167, 139, 250, 0) 100%), radial-gradient(74% 96% at 50% 88%, rgba(129, 140, 248, 0.42) 0%, rgba(129, 140, 248, 0) 100%), linear-gradient(148deg, #eef2ff 0%, #e9d5ff 52%, #ddd6fe 100%)',
  },
  {
    name: 'glow-zenith-gold',
    image:
      'radial-gradient(46% 62% at 14% 10%, rgba(253, 224, 71, 0.68) 0%, rgba(253, 224, 71, 0) 100%), radial-gradient(50% 66% at 84% 14%, rgba(251, 146, 60, 0.56) 0%, rgba(251, 146, 60, 0) 100%), radial-gradient(74% 98% at 50% 88%, rgba(254, 240, 138, 0.4) 0%, rgba(254, 240, 138, 0) 100%), linear-gradient(150deg, #fffbeb 0%, #fef3c7 50%, #fef9c3 100%)',
  },
  {
    name: 'glow-stellar-indigo',
    image:
      'radial-gradient(46% 62% at 14% 12%, rgba(129, 140, 248, 0.66) 0%, rgba(129, 140, 248, 0) 100%), radial-gradient(50% 66% at 84% 14%, rgba(99, 102, 241, 0.58) 0%, rgba(99, 102, 241, 0) 100%), radial-gradient(74% 96% at 50% 88%, rgba(165, 180, 252, 0.4) 0%, rgba(165, 180, 252, 0) 100%), linear-gradient(150deg, #eef2ff 0%, #e0e7ff 50%, #c7d2fe 100%)',
  },
  {
    name: 'glow-moonlit-coral',
    image:
      'radial-gradient(44% 60% at 12% 10%, rgba(253, 186, 116, 0.66) 0%, rgba(253, 186, 116, 0) 100%), radial-gradient(52% 68% at 86% 16%, rgba(251, 113, 133, 0.54) 0%, rgba(251, 113, 133, 0) 100%), radial-gradient(74% 96% at 50% 88%, rgba(254, 205, 211, 0.42) 0%, rgba(254, 205, 211, 0) 100%), linear-gradient(148deg, #fff7ed 0%, #ffe4e6 52%, #fef3c7 100%)',
  },
  {
    name: 'glow-quartz-nebula',
    image:
      'radial-gradient(46% 62% at 14% 10%, rgba(244, 114, 182, 0.56) 0%, rgba(244, 114, 182, 0) 100%), radial-gradient(50% 66% at 86% 14%, rgba(125, 211, 252, 0.46) 0%, rgba(125, 211, 252, 0) 100%), radial-gradient(76% 98% at 50% 88%, rgba(196, 181, 253, 0.4) 0%, rgba(196, 181, 253, 0) 100%), linear-gradient(150deg, #fdf2f8 0%, #eef2ff 52%, #e0f2fe 100%)',
  },
  {
    name: 'glow-plasma-orbit',
    image:
      'radial-gradient(48% 66% at 12% 10%, rgba(217, 70, 239, 0.56) 0%, rgba(217, 70, 239, 0) 100%), radial-gradient(52% 68% at 86% 12%, rgba(56, 189, 248, 0.44) 0%, rgba(56, 189, 248, 0) 100%), radial-gradient(78% 102% at 50% 90%, rgba(168, 85, 247, 0.32) 0%, rgba(168, 85, 247, 0) 100%), linear-gradient(150deg, #1e1b4b 0%, #312e81 48%, #0f172a 100%)',
  },
  {
    name: 'glow-aurora-pluto',
    image:
      'radial-gradient(46% 62% at 12% 10%, rgba(94, 234, 212, 0.56) 0%, rgba(94, 234, 212, 0) 100%), radial-gradient(52% 68% at 84% 14%, rgba(192, 132, 252, 0.5) 0%, rgba(192, 132, 252, 0) 100%), radial-gradient(76% 100% at 52% 90%, rgba(14, 165, 233, 0.3) 0%, rgba(14, 165, 233, 0) 100%), linear-gradient(150deg, #ecfeff 0%, #ede9fe 52%, #dbeafe 100%)',
  },
  {
    name: 'glow-nova-sky',
    image:
      'radial-gradient(46% 62% at 14% 10%, rgba(103, 232, 249, 0.66) 0%, rgba(103, 232, 249, 0) 100%), radial-gradient(50% 66% at 84% 14%, rgba(167, 139, 250, 0.5) 0%, rgba(167, 139, 250, 0) 100%), radial-gradient(74% 96% at 50% 88%, rgba(191, 219, 254, 0.42) 0%, rgba(191, 219, 254, 0) 100%), linear-gradient(148deg, #f0f9ff 0%, #dbeafe 52%, #e9d5ff 100%)',
  },
  {
    name: 'glow-eclipse-plum',
    image:
      'radial-gradient(48% 66% at 12% 10%, rgba(216, 180, 254, 0.56) 0%, rgba(216, 180, 254, 0) 100%), radial-gradient(52% 68% at 86% 14%, rgba(99, 102, 241, 0.44) 0%, rgba(99, 102, 241, 0) 100%), radial-gradient(78% 100% at 50% 90%, rgba(129, 140, 248, 0.3) 0%, rgba(129, 140, 248, 0) 100%), linear-gradient(150deg, #312e81 0%, #4c1d95 52%, #1f2937 100%)',
  },
  {
    name: 'glow-saturn-rings',
    image:
      'radial-gradient(46% 62% at 14% 12%, rgba(253, 186, 116, 0.64) 0%, rgba(253, 186, 116, 0) 100%), radial-gradient(52% 68% at 84% 14%, rgba(196, 181, 253, 0.44) 0%, rgba(196, 181, 253, 0) 100%), radial-gradient(76% 98% at 50% 88%, rgba(253, 224, 71, 0.36) 0%, rgba(253, 224, 71, 0) 100%), linear-gradient(148deg, #fff7ed 0%, #fef3c7 50%, #ede9fe 100%)',
  },
  {
    name: 'glow-neptune-frost',
    image:
      'radial-gradient(46% 62% at 12% 10%, rgba(147, 197, 253, 0.64) 0%, rgba(147, 197, 253, 0) 100%), radial-gradient(52% 68% at 86% 14%, rgba(34, 211, 238, 0.5) 0%, rgba(34, 211, 238, 0) 100%), radial-gradient(76% 100% at 50% 90%, rgba(224, 242, 254, 0.36) 0%, rgba(224, 242, 254, 0) 100%), linear-gradient(150deg, #eff6ff 0%, #dbeafe 50%, #cffafe 100%)',
  },
  {
    name: 'glow-opaline-blush',
    image:
      'radial-gradient(52% 62% at 14% 16%, color-mix(in oklch, oklch(86% 0.09 20) 80%, transparent) 0%, transparent 100%), radial-gradient(54% 66% at 86% 14%, color-mix(in oklch, oklch(83% 0.11 335) 76%, transparent) 0%, transparent 100%), radial-gradient(78% 96% at 52% 88%, color-mix(in oklch, oklch(89% 0.07 80) 70%, transparent) 0%, transparent 100%), linear-gradient(145deg, oklch(98% 0.01 20) 0%, oklch(93% 0.05 18) 34%, color-mix(in oklch, oklch(90% 0.08 340) 75%, oklch(95% 0.04 80)) 70%, oklch(96% 0.03 30) 100%)',
  },
  {
    name: 'glow-neon-lagoon',
    image:
      'radial-gradient(48% 60% at 12% 18%, color-mix(in oklch, oklch(82% 0.15 190) 84%, transparent) 0%, transparent 100%), radial-gradient(52% 64% at 86% 12%, color-mix(in oklch, oklch(74% 0.14 250) 74%, transparent) 0%, transparent 100%), radial-gradient(74% 94% at 52% 86%, color-mix(in oklch, oklch(78% 0.16 155) 68%, transparent) 0%, transparent 100%), linear-gradient(150deg, oklch(14% 0.035 252) 0%, color-mix(in oklch, oklch(24% 0.09 240) 70%, oklch(16% 0.04 210)) 42%, color-mix(in oklch, oklch(32% 0.08 200) 70%, oklch(18% 0.05 170)) 72%, oklch(20% 0.04 180) 100%)',
  },
  {
    name: 'glow-pastel-limewash',
    image:
      'radial-gradient(46% 58% at 16% 14%, color-mix(in oklch, oklch(92% 0.11 105) 86%, transparent) 0%, transparent 100%), radial-gradient(52% 64% at 84% 14%, color-mix(in oklch, oklch(88% 0.08 165) 70%, transparent) 0%, transparent 100%), radial-gradient(78% 96% at 50% 88%, color-mix(in oklch, oklch(83% 0.12 135) 66%, transparent) 0%, transparent 100%), linear-gradient(148deg, oklch(97% 0.016 98) 0%, color-mix(in oklch, oklch(92% 0.11 110) 56%, white) 36%, color-mix(in oklch, oklch(88% 0.09 160) 66%, white) 70%, oklch(94% 0.04 200) 100%)',
  },
  {
    name: 'glow-lilac-glass',
    image:
      'radial-gradient(48% 60% at 15% 14%, color-mix(in oklch, oklch(84% 0.1 315) 70%, transparent) 0%, transparent 100%), radial-gradient(54% 66% at 84% 16%, color-mix(in oklch, oklch(81% 0.08 265) 68%, transparent) 0%, transparent 100%), linear-gradient(150deg, color-mix(in oklch, oklch(98% 0.008 300) 88%, white) 0%, color-mix(in oklch, oklch(92% 0.04 300) 80%, white) 40%, color-mix(in oklch, oklch(90% 0.05 260) 72%, white) 100%)',
  },
  {
    name: 'glow-mango-soda',
    image:
      'radial-gradient(46% 62% at 14% 14%, color-mix(in oklch, oklch(88% 0.14 55) 78%, transparent) 0%, transparent 100%), radial-gradient(52% 66% at 84% 14%, color-mix(in oklch, oklch(80% 0.15 20) 72%, transparent) 0%, transparent 100%), radial-gradient(74% 96% at 50% 88%, color-mix(in oklch, oklch(90% 0.09 95) 66%, transparent) 0%, transparent 100%), linear-gradient(148deg, oklch(98% 0.012 60) 0%, oklch(92% 0.08 55) 46%, oklch(88% 0.11 30) 78%, oklch(94% 0.04 95) 100%)',
  },
]

/** FNV-1a, 32 бита. Нужен только ровный разброс, не криптостойкость. */
function hash(seed: string): number {
  let value = 0x811c9dc5
  for (let index = 0; index < seed.length; index += 1) {
    value ^= seed.charCodeAt(index)
    value = Math.imul(value, 0x01000193)
  }
  return value >>> 0
}

/** Градиент-заглушка для семени. Одно семя — всегда один и тот же градиент. */
export function glowGradient(seed: string): string {
  const palette = GLOW_GRADIENTS
  const picked = palette[hash(seed) % palette.length]
  // Палитра непустая, но `noUncheckedIndexedAccess` об этом не знает.
  return picked?.image ?? palette[0]?.image ?? 'none'
}
