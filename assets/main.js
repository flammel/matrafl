document.querySelectorAll("[data-confirm-delete]").forEach((form) => {
  form.addEventListener("submit", (e) => {
    e.preventDefault();
    if (confirm("Are you sure you want to delete this?")) {
      form.submit();
    }
  });
});

document.querySelectorAll("[data-toggler]").forEach((toggler) => {
  toggler.addEventListener("click", () => {
    const selector = toggler.getAttribute("data-toggler-target");
    if (!selector) {
      console.warn("Toggler has no target selector", toggler);
      return;
    }
    const target = document.querySelector(selector);
    if (!target) {
      console.warn("Toggler target not found", selector);
      return;
    }
    target.toggleAttribute("hidden");
  });
});

document.querySelectorAll("[data-search-trigger]").forEach((input) => {
  input.addEventListener("input", () => {
    const value = input.value.toLowerCase().trim();
    document.querySelectorAll("[data-search-item]").forEach((item) => {
      const text = item.textContent.toLowerCase();
      if (text.includes(value)) {
        item.removeAttribute("hidden");
      } else {
        item.setAttribute("hidden", true);
      }
    });
  });
});

document.querySelectorAll("[data-consumable-select-trigger]").forEach((button) => {
  button.addEventListener("click", function () {
    document.querySelector("[data-consumable-select-dialog]").showModal();
  });
});

document.querySelectorAll("[data-consumable-select-dialog]").forEach((dialog) => {
  const closeButton = dialog.querySelector("[data-consumable-select-closer]");
  closeButton.addEventListener("click", () => {
    closeButton.closest("dialog").close();
  });

  const searchInput = dialog.querySelector("input");
  searchInput.addEventListener("input", () => {
    const value = searchInput.value.toLowerCase().trim();
    let lastVisible = null;
    dialog.querySelectorAll(".option").forEach((item) => {
      const text = item.textContent.toLowerCase();
      item.classList.remove("filtered-last-child");
      if (text.includes(value)) {
        item.removeAttribute("hidden");
        lastVisible = item;
      } else {
        item.setAttribute("hidden", true);
      }
    });
    if (lastVisible) {
      lastVisible.classList.add("filtered-last-child");
    }
  });

  dialog.querySelectorAll(".option").forEach((option) => {
    option.addEventListener("click", () => {
      const consumableId = option.getAttribute("data-consumable-id");
      const idInput = document.querySelector("[data-consumable-select-id-input]");
      idInput.value = consumableId;
      
      const consumableType = option.getAttribute("data-consumable-type");
      const typeInput = document.querySelector("[data-consumable-select-type-input]");
      typeInput.value = consumableType;

      const consumableName = option.getAttribute("data-consumable-name");
      const trigger = document.querySelector("[data-consumable-select-trigger]");
      trigger.textContent = consumableName;

      const consumableUrl = option.getAttribute("data-consumable-url");
      const opener = document.querySelector("[data-consumable-select-open]");
      opener.setAttribute('href', consumableUrl);

      dialog.close();
    });
  });
});

if ("serviceWorker" in navigator) {
  navigator.serviceWorker.register("/assets/service-worker.js");
}
