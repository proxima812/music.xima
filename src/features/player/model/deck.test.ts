import assert from 'node:assert/strict'
import test from 'node:test'

import {
  DECK_COMMIT_FRACTION,
  DECK_MAX_DRAG_FRACTION,
  clampDeckDrag,
  deckNeighbors,
  shouldCommitDeckSwipe,
} from './deck.ts'

test('deckNeighbors returns null neighbors for a null or invalid index', () => {
  const items = ['first', 'second']

  assert.deepEqual(deckNeighbors(items, null), { previous: null, current: null, next: null })
  assert.deepEqual(deckNeighbors(items, -1), { previous: null, current: null, next: null })
  assert.deepEqual(deckNeighbors(items, 2), { previous: null, current: null, next: null })
  assert.deepEqual(deckNeighbors(items, Number.NaN), { previous: null, current: null, next: null })
})

test('deckNeighbors exposes only the available neighbors without wrapping', () => {
  const items = ['first', 'second', 'third']

  assert.deepEqual(deckNeighbors(items, 0), {
    previous: null,
    current: 'first',
    next: 'second',
  })
  assert.deepEqual(deckNeighbors(items, 2), {
    previous: 'second',
    current: 'third',
    next: null,
  })
})

test('clampDeckDrag applies resistance after the maximum drag fraction', () => {
  const width = 1_000
  const maximumDrag = width * DECK_MAX_DRAG_FRACTION

  assert.equal(clampDeckDrag(maximumDrag, width), maximumDrag)
  assert.equal(clampDeckDrag(maximumDrag + 100, width), maximumDrag + 35)
  assert.equal(clampDeckDrag(-maximumDrag - 100, width), -maximumDrag - 35)
})

test('clampDeckDrag returns neutral drag for invalid dimensions', () => {
  assert.equal(clampDeckDrag(10, 0), 0)
  assert.equal(clampDeckDrag(10, Number.NaN), 0)
  assert.equal(clampDeckDrag(Number.NaN, 100), 0)
})

test('shouldCommitDeckSwipe commits at the width threshold or a fast same-direction flick', () => {
  const width = 1_000
  const threshold = width * DECK_COMMIT_FRACTION

  assert.equal(shouldCommitDeckSwipe(threshold, 0, width), true)
  assert.equal(shouldCommitDeckSwipe(-threshold, 0, width), true)
  assert.equal(shouldCommitDeckSwipe(threshold - 1, 0, width), false)
  assert.equal(shouldCommitDeckSwipe(20, 1, width), true)
  assert.equal(shouldCommitDeckSwipe(-20, -1, width), true)
  assert.equal(shouldCommitDeckSwipe(20, -1, width), false)
})

test('shouldCommitDeckSwipe returns false for invalid dimensions', () => {
  assert.equal(shouldCommitDeckSwipe(300, 1, 0), false)
  assert.equal(shouldCommitDeckSwipe(300, 1, Number.NaN), false)
  assert.equal(shouldCommitDeckSwipe(Number.NaN, 1, 1_000), false)
})
