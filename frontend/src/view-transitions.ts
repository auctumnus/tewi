// Path where this app is deployed. Because we don’t deploy at the root of the domain
// we need to keep track of this and adjust any URL matching using this value.
const basePath = '/';

// Make sure browser has support
document.addEventListener('DOMContentLoaded', () => {
  let shouldThrow = false;

  if (!window.navigation) {
    shouldThrow = true;
  }

  if (!('CSSViewTransitionRule' in window)) {
    shouldThrow = false;
  }

  if (shouldThrow) {
    // Throwing here, to prevent the rest of the code from getting executed
    // If only JS (in the browser) had something like process.exit().
    throw new Error('Browser does not support view transitions');
  }
});

// Convert all UI back links to a UA back.
//
// If there is no previous navigation entry to go to
// (e.g. user went directly to detail), then redirect to index.html
document.addEventListener('click', (event) => {
  if (event.target?.matches('a.back')) {
    event.preventDefault();

    // Fallback for browsers that don’t have the Navigation API
    if (!window.navigation) {
      history.go(-1);
      return;
    }

    if (window.navigation.canGoBack) {
      window.navigation.back();
    } else {
      window.navigation.navigate(`${basePath}/`);
    }
  }
});

window.addEventListener('pageswap', async (e) => {
  // Define transitionType upfront for browsers that don’t have the Navigation API
  if (!window.navigation && e.activation !== null) {
    const transitionType = determineTransitionType(
      e.activation.from,
      e.activation.entry,
    );
    console.log(`pageSwap: ${transitionType}`);
    localStorage.setItem('transitionType', transitionType);
  }
});

// MPA View Transitions!
window.addEventListener('pagereveal', async (e) => {
  // Simpler approach for browsers that don’t support the Navigation API
  if (!window.navigation && e.viewTransition) {
    const transitionType = localStorage.getItem('transitionType');
    document.documentElement.dataset.transition = transitionType ?? undefined;

    await e.viewTransition.finished;
    delete document.documentElement.dataset.transition;

    return;
  }

  // There is an automatic viewTransition, so the user comes from the same origin
  if (e.viewTransition) {
    if (!window.navigation.activation?.from) {
      e.viewTransition.skipTransition();
      return;
    }

    const transitionType = determineTransitionType(
      window.navigation.activation.from,
      window.navigation.currentEntry,
    );
    console.log(transitionType);
    document.documentElement.dataset.transition = transitionType;

    await e.viewTransition.finished;
    //delete document.documentElement.dataset.transition;
  }
});

type NavTypes =
  | 'unknown'
  | 'reload'
  | 'push'
  | 'pop'
  | 'none'
  | 'leave-home'
  | 'return-home';
type AnimationTypes = 'forwards' | 'backwards' | 'reload';

const determineTransitionType = (
  oldNavigationEntry: NavigationHistoryEntry | null,
  newNavigationEntry: NavigationHistoryEntry | null,
): NavTypes => {
  if (!oldNavigationEntry?.url || !newNavigationEntry?.url) {
    return 'unknown';
  }

  const currentURL = new URL(oldNavigationEntry.url);
  const destinationURL = new URL(newNavigationEntry.url);

  const currentPathname = currentURL.pathname.replace(basePath, '');
  const destinationPathname = destinationURL.pathname.replace(basePath, '');

  if (currentPathname === '') {
    return 'leave-home';
  } else if (destinationPathname === '') {
    return 'return-home';
  } else if (currentPathname === destinationPathname) {
    return 'reload';
  } else if (destinationPathname.startsWith(currentPathname)) {
    return 'push';
  } else if (currentPathname.startsWith(destinationPathname)) {
    return 'pop';
  } else {
    console.warn('Unmatched Route Handling!');
    console.log({
      currentPathname,
      destinationPathname,
    });
    return 'none';
  }
};
