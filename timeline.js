const repos = [
  'OoTRandomizer/OoT-Randomizer',
  'comex/rust-shlex',
  'dasgefolge/gefolge.org',
  'fenhl/OoT-Randomizer',
  'fenhl/webloc-cli',
  'midoshouse/ootr-multiworld',
];

function loadTimeline() {
  let repo = document.getElementById("reposelect").value;
  let label = document.getElementById("labelselect").value;
  if (label === '') {
    label = null;
  }

  const params = new URLSearchParams(location.search);
  params.set('repo', repo);
  if (label === null) {
    params.delete('label');
  } else {
    params.set('label', label);
  }
  window.history.replaceState({}, '', `${location.pathname}?${params.toString()}`);

  fetch('data/' + repo + '.json')
  .then(
    function(response) {
      if (response.status !== 200) {
        console.log('Looks like there was a problem. Status Code: ' +
          response.status);
        return;
      }

      response.text().then(function(respBody) {
        let timelineData = JSON.parse(respBody, JSON.dateParser);

        let labels = timelineData['labels'];

        if (!populateLabels(labels, label)) {
          label = null;
        }

        let timeline = timelineData['timeline'];

        populateGraph(timeline, repo, label);
      });
    }
  )
  .catch(function(err) {
    console.log('Fetch Error', err);
  });
}

function populateLabels(labels, label) {
  let select = document.getElementById("labelselect");

  let firstEl = null;
  let anySelected = false;
  for(var i = 0; i < labels.length; i++) {
    let opt = labels[i];
    let el = document.createElement("option");
    el.textContent = opt;
    el.value = opt;
    if (label !== null && label != '' && opt == label) {
      el.selected = true;
      anySelected = true;
    }
    select.appendChild(el);
    if (firstEl === null) {
      firstEl = el;
    }
  }
  let el = document.createElement("option");
  el.textContent = '(any)';
  el.value = '';
  if (label === null || label === '' || !anySelected) {
    el.selected = true;
  }
  select.insertBefore(el, firstEl);
  return anySelected;
}

function populateGraph(timeline, repo, label) {
  var issues = {
    type: "scatter",
    name: 'Issues',
    x: timeline.map(a => a.day),
    y: timeline.map(a => (label === null ? a['open_issues'] : a['issue_labels'][label] || 0)),
  }
  var prs = {
    type: "scatter",
    name: 'PRs',
    x: timeline.map(a => a.day),
    y: timeline.map(a => (label === null ? a['open_prs'] : a['pr_labels'][label] || 0)),
  }
  var total = {
    type: "scatter",
    name: 'Total',
    x: timeline.map(a => a.day),
    y: timeline.map(a => (label === null ? a['open_issues'] + a['open_prs'] : (a['issue_labels'][label] || 0) + (a['pr_labels'][label] || 0))),
  }
  let layout = {
    title: {
      text: '<a href="https://github.com/' + repo + '">' + repo + '</a>' + (label === null ? '' : ': ' + label)
    },
    showSendToCloud:false,
    autosize: true,
    xaxis: {
      tickformat: '%Y-%m-%d',
    },
  };
  var data = [
    issues,
    prs,
    total,
  ];
  Plotly.newPlot('graph', data, layout, {displayModeBar: false});
}

document.addEventListener('DOMContentLoaded', function() {
  let select = document.getElementById("reposelect");
  const urlParams = new URLSearchParams(window.location.search);
  const repo = urlParams.get('repo');

  for(var i = 0; i < repos.length; i++) {
    let opt = repos[i];
    let el = document.createElement("option");
    el.textContent = opt;
    el.value = opt;
    if (repo != '' && opt == repo) {
      el.selected = true;
    }
    select.appendChild(el);
  }
  loadTimeline();
})
