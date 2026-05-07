# -*- coding: utf-8 -*-
import re
import webbrowser
import ipywidgets as widgets
import urllib.parse
from pathlib import Path
from icrawler.builtin import GoogleImageCrawler
from PIL import Image
ROOT_PATH = Path(__file__).parent.parent


class NamePrompt:
    def __init__(
            self, item, callback, guessed_name=None, name_proposals=None, similar_names=None, manufacturers=None):
        """IPyWidgets UI element for setting name for a crawled headphone"""
        self._item = item
        self.callback = callback
        self._guessed_name = guessed_name or item.source_name or ''
        self._guessed_name = self._guessed_name.strip()
        self._name_proposals = name_proposals if name_proposals is not None else []
        self._similar_names = similar_names if similar_names is not None else []
        self._manufacturers = manufacturers
        # UI elements
        self.search_button = widgets.Button(description='🔎', layout=widgets.Layout(width='48px', height='44px'))
        self.search_button.on_click(self.handle_search)
        self._name_proposal_buttons = []
        self.text_field = widgets.Text(value=self.guessed_name, layout=widgets.Layout(width='400px'))
        self.form_buttons = []
        for form in ['over-ear', 'in-ear', 'earbud', 'ignore']:
            btn = widgets.Button(description=form, layout=widgets.Layout(width='80px'))
            btn.on_click(self.handle_form_click)
            self.form_buttons.append(btn)
        self._imgs = widgets.HBox([])
        self.widget = None
        self.reload_ui()

    def reload_ui(self):
        for button in self.form_buttons:
            button.button_style = 'success' if button.description == self.item.form else (
                'danger' if button.description == 'ignore' else 'warning')
        self.widget = widgets.VBox([
            widgets.HTML(
                value=f'<i style="text-align: center; display: inline-block; width: 100%; line-height: 1">'
                      f'{urllib.parse.unquote(self.item.url) or self.item.source_name}</i>'),
            *self._name_proposal_buttons,  # Name suggestions
            widgets.HTML(
                f'<h4 style="margin: 0; text-align: center; line-height: 1.7; font-size: 24px">{self.name}</h4>',
            layout=widgets.Layout(width='400px')),
            self.text_field,
            widgets.HBox([*self.form_buttons]),
            widgets.HBox([
                widgets.Label(f'Measured on: {self.item.rig}'),
                self.search_button
            ]),
            self._imgs
        ])

    @property
    def item(self):
        return self._item

    @item.setter
    def item(self, item):
        self._item = item
        self.reload_ui()

    @property
    def guessed_name(self):
        return self._guessed_name

    @guessed_name.setter
    def guessed_name(self, value):
        if self.guessed_name == self.text_field.value:
            # User hasn't updated text field value manually, safe to update to new guessed name
            self._guessed_name = value.strip()
            self.text_field.value = self._guessed_name
            self.reload_ui()

    @property
    def name_proposals(self):
        return self._name_proposals

    @name_proposals.setter
    def name_proposals(self, name_proposals):
        self._name_proposals = name_proposals if name_proposals is not None else []
        # Add button for each name proposal
        self._name_proposal_buttons = []
        if self._name_proposals is not None:
            for item in name_proposals.items:
                btn = widgets.Button(
                    description=f'{item.name}',
                    button_style='success' if self.guessed_name and item.name.lower() == self._manufacturers.replace(self.guessed_name).lower() else 'primary',
                    layout=widgets.Layout(width='400px'))
                btn.on_click(self.handle_name_proposal_click)
                self._name_proposal_buttons.append(btn)
            self._name_proposal_buttons = self._name_proposal_buttons[::-1]
            self.reload_ui()

    @property
    def similar_names(self):
        return self._similar_names

    @similar_names.setter
    def similar_names(self, similar_names):
        self._similar_names = similar_names if similar_names is not None else []
        if self._similar_names:
            self.reload_ui()

    @property
    def name(self):
        return self.guessed_name or urllib.parse.unquote(self.item.url.split('/')[-1])

    def create_query(self):
        return re.sub(r'\s\(.*\)$', '', self.text_field.value).strip()

    def handle_search(self, btn):
        self.reload_ui()
        quoted = urllib.parse.quote_plus(self.create_query())
        url = f'https://google.com/search?q={quoted}&tbm=isch'
        webbrowser.open(url)
        self.search_images()

    def search_images(self):
        if not self.text_field.value:
            return
        img_dir = ROOT_PATH.joinpath(
            'dbtools', 'google-image-search', self.create_query())
        if not img_dir.exists():
            google_crawler = GoogleImageCrawler(storage={'root_dir': img_dir})
            google_crawler.crawl(keyword=img_dir.name, max_num=4)
        # Create image widgets
        images = []
        for fp in img_dir.glob('*'):
            with open(fp, 'rb') as fh:
                im = Image.open(fp)
                xsize, ysize = im.size
                images.append(widgets.Image(value=fh.read(), width=200 * xsize / ysize, height=200))
        self._imgs.children = images

    def handle_name_proposal_click(self, btn):
        btn.button_style = 'success'
        item = self.item.copy()
        proposal_item = self.name_proposals.find_one(name=btn.description).copy()
        item.name = proposal_item.name
        item.form = proposal_item.form
        self.callback(item)

    def handle_form_click(self, btn):
        item = self.item.copy()
        item.name = self.text_field.value.strip()
        item.form = btn.description
        self.callback(item)
